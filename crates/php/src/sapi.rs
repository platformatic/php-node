use std::{
  collections::HashMap,
  env::current_exe,
  ffi::{c_char, c_int, c_void, CStr},
  sync::{Arc, RwLock, Weak},
};

use bytes::Buf;

use ext_php_rs::{
  builders::SapiBuilder,
  embed::SapiModule,
  exception::register_error_observer,
  ffi::{
    ext_php_rs_sapi_per_thread_init, ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup,
    php_module_shutdown, php_module_startup, php_register_variable, sapi_send_headers,
    sapi_shutdown, sapi_startup, ZEND_RESULT_CODE_SUCCESS,
  },
  prelude::*,
  zend::{SapiGlobals, SapiHeader},
};

use once_cell::sync::OnceCell;

use crate::{
  strings::{cstr, drop_str, maybe_current_dir},
  EmbedException, RequestContext,
};
use lang_handler::Header;

// This is a helper to ensure that PHP is initialized and deinitialized at the
// appropriate times.
#[derive(Debug)]
pub(crate) struct Sapi(RwLock<Box<SapiModule>>);

impl Sapi {
  pub fn new() -> Result<Self, EmbedException> {
    let mut sapi = SapiBuilder::new("php_lang_handler", "PHP Lang Handler")
      .startup_function(sapi_module_startup)
      .shutdown_function(sapi_module_shutdown)
      // .activate_function(sapi_module_activate)
      .deactivate_function(sapi_module_deactivate)
      .ub_write_function(sapi_module_ub_write)
      .flush_function(sapi_module_flush)
      .send_header_function(sapi_module_send_header)
      .read_post_function(sapi_module_read_post)
      .read_cookies_function(sapi_module_read_cookies)
      .register_server_variables_function(sapi_module_register_server_variables)
      .log_message_function(sapi_module_log_message)
      // .executable_location(args.get(0))
      .build()
      .map_err(|_| EmbedException::SapiNotInitialized)?;

    sapi.ini_defaults = Some(sapi_cli_ini_defaults);
    sapi.php_ini_path_override = std::ptr::null_mut();
    sapi.php_ini_ignore_cwd = 1;
    sapi.additional_functions = std::ptr::null();
    // sapi.phpinfo_as_text = 1;

    let exe_loc = current_exe()
      .map(|p| p.display().to_string())
      .map_err(|_| EmbedException::FailedToFindExeLocation)?;

    sapi.executable_location = cstr(exe_loc)?;
    let mut boxed = Box::new(sapi);

    unsafe {
      ext_php_rs_sapi_startup();
      sapi_startup(boxed.as_mut());

      if let Some(startup) = boxed.startup {
        startup(boxed.as_mut());
      }
    }

    // TODO: Should maybe capture this to store in EmbedException rather than
    // writing to the ResponseBuilder here. When php_execute_script fails it
    // should read that and could return an error or write it to the
    // ResponseBuilder there.
    register_error_observer(|_error_type, _file, _line, message| {
      message
        .as_str()
        .inspect(|msg| {
          if let Some(ctx) = RequestContext::current() {
            ctx.response_builder().exception(*msg);
          }
        })
        // TODO: Report this error somehow?
        .ok();
    });

    Ok(Sapi(RwLock::new(boxed)))
  }

  pub fn startup<'a>(&'a self) -> Result<(), EmbedException> {
    unsafe {
      ext_php_rs_sapi_per_thread_init();
    }

    let rwlock = &self.0;
    let sapi = rwlock.read().map_err(|_| EmbedException::SapiLockFailed)?;

    if let Some(startup) = sapi.startup {
      if unsafe { startup(sapi.into_raw()) } != ZEND_RESULT_CODE_SUCCESS {
        return Err(EmbedException::SapiNotStarted);
      }
    }

    Ok(())
  }
}

impl Drop for Sapi {
  fn drop(&mut self) {
    unsafe {
      sapi_shutdown();
      ext_php_rs_sapi_shutdown();
    }
  }
}

pub(crate) static SAPI_INIT: OnceCell<RwLock<Weak<Sapi>>> = OnceCell::new();

pub fn ensure_sapi() -> Result<Arc<Sapi>, EmbedException> {
  let weak_sapi = SAPI_INIT.get_or_try_init(|| Ok(RwLock::new(Weak::new())))?;

  if let Some(sapi) = weak_sapi
    .read()
    .map_err(|_| EmbedException::SapiLockFailed)?
    .upgrade()
  {
    return Ok(sapi);
  }

  let mut rwlock = weak_sapi
    .write()
    .map_err(|_| EmbedException::SapiLockFailed)?;

  let sapi = Sapi::new().map(Arc::new)?;
  *rwlock = Arc::downgrade(&sapi);

  Ok(sapi)
}

//
// Sapi functions
//

// error_reporting = E_ERROR | E_WARNING | E_PARSE | E_CORE_ERROR | E_CORE_WARNING | E_COMPILE_ERROR | E_COMPILE_WARNING | E_RECOVERABLE_ERROR
static HARDCODED_INI: &str = "
  error_reporting=4343
  ignore_repeated_errors=1
  display_errors=0
  display_startup_errors=0
  register_argc_argv=1
  log_errors=1
  implicit_flush=0
  memory_limit=128M
  output_buffering=0
  enable_post_data_reading=1
  html_errors=0
	max_execution_time=0
	max_input_time=-1
";

#[no_mangle]
pub extern "C" fn sapi_cli_ini_defaults(ht: *mut ext_php_rs::types::ZendHashTable) {
  let config = unsafe { &mut *ht };

  let ini_lines = str::trim(HARDCODED_INI).lines().map(str::trim);

  for line in ini_lines {
    if let Some((key, value)) = line.split_once('=') {
      use ext_php_rs::convert::IntoZval;
      let value = value.into_zval(true).unwrap();
      // TODO: Capture error somehow?
      config.insert(key, value).ok();
    }
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_startup(
  sapi_module: *mut SapiModule,
) -> ext_php_rs::ffi::zend_result {
  unsafe { php_module_startup(sapi_module, get_module()) }
}

#[no_mangle]
pub extern "C" fn sapi_module_shutdown(
  _sapi_module: *mut SapiModule,
) -> ext_php_rs::ffi::zend_result {
  unsafe {
    php_module_shutdown();
  }
  ZEND_RESULT_CODE_SUCCESS
}

#[no_mangle]
pub extern "C" fn sapi_module_deactivate() -> c_int {
  {
    let mut globals = SapiGlobals::get_mut();

    for i in 0..globals.request_info.argc {
      drop_str(unsafe { *globals.request_info.argv.offset(i as isize) });
    }

    globals.server_context = std::ptr::null_mut();
    globals.request_info.argc = 0;
    globals.request_info.argv = std::ptr::null_mut();

    drop_str(globals.request_info.request_method);
    drop_str(globals.request_info.query_string);
    drop_str(globals.request_info.request_uri);
    drop_str(globals.request_info.path_translated);
    drop_str(globals.request_info.content_type);
    drop_str(globals.request_info.cookie_data);
    // drop_str(globals.request_info.php_self);
    drop_str(globals.request_info.auth_user);
    drop_str(globals.request_info.auth_password);
    drop_str(globals.request_info.auth_digest);
  }

  // TODO: When _is_ it safe to reclaim the request context?
  RequestContext::reclaim();

  ZEND_RESULT_CODE_SUCCESS
}

#[no_mangle]
pub extern "C" fn sapi_module_ub_write(str: *const i8, str_length: usize) -> usize {
  if str.is_null() || str_length == 0 {
    return 0;
  }
  let bytes = unsafe { std::slice::from_raw_parts(str as *const u8, str_length) };
  let len = bytes.len();
  if let Some(ctx) = RequestContext::current() {
    ctx.response_builder().body_write(bytes);
  }
  len
}

#[no_mangle]
pub extern "C" fn sapi_module_flush(_server_context: *mut c_void) {
  if let Some(ctx) = RequestContext::current() {
    unsafe { sapi_send_headers() };
    let mut globals = SapiGlobals::get_mut();
    globals.headers_sent = 1;
    ctx
      .response_builder()
      .status(globals.sapi_headers.http_response_code);
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_send_header(header: *mut SapiHeader, _server_context: *mut c_void) {
  // Not sure _why_ this is necessary, but it is.
  if header.is_null() {
    return;
  }

  let header = unsafe { &*header };
  let name = header.name();

  // Header value is None for http version + status line
  if let Some(value) = header.value() {
    if let Some(ctx) = RequestContext::current() {
      ctx.response_builder().header(name, value);
    }
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_read_post(buffer: *mut c_char, length: usize) -> usize {
  if length == 0 {
    return 0;
  }

  RequestContext::current()
    .map(|ctx| ctx.request().body())
    .map(|body| {
      let length = length.min(body.len());
      if length == 0 {
        return 0;
      }

      let chunk = body.take(length);

      unsafe {
        std::ptr::copy_nonoverlapping(chunk.chunk().as_ptr() as *mut c_char, buffer, length);
      }
      length
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn sapi_module_read_cookies() -> *mut c_char {
  SapiGlobals::get().request_info.cookie_data
}

#[no_mangle]
pub extern "C" fn sapi_module_register_server_variables(vars: *mut ext_php_rs::types::Zval) {
  unsafe {
    // use ext_php_rs::ffi::php_import_environment_variables;
    // if let Some(f) = php_import_environment_variables {
    //   f(vars);
    // }

    RequestContext::current()
      .map(|ctx| ctx.request())
      // Convert to a result so we can use and_then with ? syntax...
      .ok_or(EmbedException::RequestContextUnavailable)
      .and_then(|request| {
        let headers = request.headers();

        for (key, values) in headers.iter() {
          let maybe_header = match values {
            Header::Single(header) => Some(header),
            Header::Multiple(headers) => headers.first(),
          };

          if let Some(header) = maybe_header {
            let cgi_key = format!("HTTP_{}", key.to_ascii_uppercase().replace("-", "_"));
            php_register_variable(cstr(&cgi_key)?, cstr(header)?, vars);
          }
        }

        let globals = SapiGlobals::get();
        let req_info = &globals.request_info;

        let cwd = maybe_current_dir()?;
        let cwd_cstr = cstr(cwd.display().to_string())?;

        let script_filename = req_info.path_translated;
        let script_name = if !req_info.request_uri.is_null() {
          req_info.request_uri
        } else {
          c"".as_ptr()
        };

        php_register_variable(cstr("REQUEST_SCHEME")?, cstr(request.url().scheme())?, vars);
        php_register_variable(cstr("CONTEXT_PREFIX")?, cstr("")?, vars);
        php_register_variable(cstr("SERVER_ADMIN")?, cstr("webmaster@localhost")?, vars);
        php_register_variable(cstr("GATEWAY_INTERFACE")?, cstr("CGI/1.1")?, vars);

        php_register_variable(cstr("PHP_SELF")?, script_name, vars);
        php_register_variable(cstr("SCRIPT_NAME")?, script_name, vars);
        php_register_variable(cstr("SCRIPT_FILENAME")?, script_filename, vars);
        php_register_variable(cstr("PATH_TRANSLATED")?, script_filename, vars);
        php_register_variable(cstr("DOCUMENT_ROOT")?, cwd_cstr, vars);
        php_register_variable(cstr("CONTEXT_DOCUMENT_ROOT")?, cwd_cstr, vars);

        if let Ok(server_name) = hostname::get() {
          if let Some(server_name) = server_name.to_str() {
            php_register_variable(cstr("SERVER_NAME")?, cstr(server_name)?, vars);
          }
        }

        if !req_info.request_uri.is_null() {
          php_register_variable(cstr("REQUEST_URI")?, req_info.request_uri, vars);
        }

        php_register_variable(cstr("SERVER_PROTOCOL")?, cstr("HTTP/1.1")?, vars);

        let sapi = SAPI_INIT.get().ok_or(EmbedException::SapiNotInitialized)?;

        php_register_variable(
          cstr("SERVER_SOFTWARE")?,
          sapi
            .read()
            .map_err(|_| EmbedException::SapiLockFailed)?
            .upgrade()
            .ok_or(EmbedException::SapiLockFailed)?
            .0
            .read()
            .map_err(|_| EmbedException::SapiLockFailed)?
            .name,
          vars,
        );

        if let Some(info) = request.local_socket() {
          php_register_variable(cstr("SERVER_ADDR")?, cstr(info.ip().to_string())?, vars);
          php_register_variable(cstr("SERVER_PORT")?, cstr(info.port().to_string())?, vars);
        }

        if let Some(info) = request.remote_socket() {
          php_register_variable(cstr("REMOTE_ADDR")?, cstr(info.ip().to_string())?, vars);
          php_register_variable(cstr("REMOTE_PORT")?, cstr(info.port().to_string())?, vars);
        }

        if !req_info.request_method.is_null() {
          php_register_variable(cstr("REQUEST_METHOD")?, req_info.request_method, vars);
        }

        if !req_info.cookie_data.is_null() {
          php_register_variable(cstr("HTTP_COOKIE")?, req_info.cookie_data, vars);
        }

        if !req_info.query_string.is_null() {
          php_register_variable(cstr("QUERY_STRING")?, req_info.query_string, vars);
        }

        Ok(())
      })
      // TODO: Capture errors somehow so we can surface them...
      .ok();
  };
}

#[no_mangle]
pub extern "C" fn sapi_module_log_message(message: *const c_char, _syslog_type_int: c_int) {
  let message = unsafe { CStr::from_ptr(message) };
  if let Some(ctx) = RequestContext::current() {
    ctx.response_builder().log_write(message.to_bytes());
  }
}

//
// PHP Module
//

#[php_function]
pub fn apache_request_headers() -> Result<HashMap<String, String>, String> {
  let mut headers = HashMap::new();

  let request = RequestContext::current()
    .map(|ctx| ctx.request())
    .ok_or("Request context unavailable")?;

  for (key, value) in request.headers().iter() {
    headers.insert(key.to_string(), value.into());
  }

  Ok(headers)
}

#[php_module]
pub fn module(module: ModuleBuilder<'_>) -> ModuleBuilder<'_> {
  module.function(wrap_function!(apache_request_headers))
}

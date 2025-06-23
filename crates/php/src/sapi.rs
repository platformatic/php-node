use std::{
  collections::HashMap,
  env::current_exe,
  ffi::{c_char, c_int, c_void, CStr},
  sync::{Arc, RwLock, Weak},
};


use ext_php_rs::{
  alloc::{efree, estrdup},
  builders::SapiBuilder,
  embed::SapiModule,
  // exception::register_error_observer,
  ffi::{
    ext_php_rs_sapi_per_thread_init, ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup,
    php_module_shutdown, php_module_startup, php_register_variable, sapi_send_headers,
    sapi_shutdown, sapi_startup, ZEND_RESULT_CODE_SUCCESS,
  },
  prelude::*,
  zend::{SapiGlobals, SapiHeader},
};

use once_cell::sync::OnceCell;

use crate::{EmbedRequestError, EmbedStartError, RequestContext};

// This is a helper to ensure that PHP is initialized and deinitialized at the
// appropriate times.
#[derive(Debug)]
pub(crate) struct Sapi(RwLock<Box<SapiModule>>);

impl Sapi {
  pub fn new() -> Result<Self, EmbedStartError> {
    let exe_loc = current_exe()
      .map(|p| p.display().to_string())
      .map_err(|_| EmbedStartError::ExeLocationNotFound)?;

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
      .executable_location(&exe_loc)
      .build()
      .map_err(|_| EmbedStartError::SapiNotInitialized)?;

    sapi.ini_defaults = Some(sapi_cli_ini_defaults);
    sapi.php_ini_path_override = std::ptr::null_mut();
    sapi.php_ini_ignore_cwd = 1;
    sapi.additional_functions = std::ptr::null();
    // sapi.phpinfo_as_text = 1;

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
    // TODO: Having error observers registered crashes Laravel.
    // register_error_observer(|error_type, file, line, message| {
    //   let file = file.as_str().expect("should convert zend_string to str");
    //   // TODO: Report this error somehow?
    //   if let Ok(msg) = message.as_str() {
    //     println!("PHP Error #{}: {}\n\tfrom {}:{}", error_type, msg, file, line);
    //     if let Some(ctx) = RequestContext::current() {
    //       ctx.response_builder().exception(msg);
    //     }
    //   }
    // });

    Ok(Sapi(RwLock::new(boxed)))
  }

  pub fn startup(&self) -> Result<(), EmbedRequestError> {
    unsafe {
      ext_php_rs_sapi_per_thread_init();
    }

    let sapi = &mut self
      .0
      .write()
      .map_err(|_| EmbedRequestError::SapiNotStarted)?;

    if let Some(startup) = sapi.startup {
      if unsafe { startup(sapi.as_mut()) } != ZEND_RESULT_CODE_SUCCESS {
        return Err(EmbedRequestError::SapiNotStarted);
      }
    }

    Ok(())
  }

  pub fn shutdown(&self) -> Result<(), EmbedRequestError> {
    let sapi = &mut self
      .0
      .write()
      .map_err(|_| EmbedRequestError::SapiNotShutdown)?;

    if let Some(shutdown) = sapi.shutdown {
      if unsafe { shutdown(sapi.as_mut()) } != ZEND_RESULT_CODE_SUCCESS {
        return Err(EmbedRequestError::SapiNotShutdown);
      }
    }

    Ok(())
  }
}

impl Drop for Sapi {
  fn drop(&mut self) {
    self.shutdown().unwrap();

    unsafe {
      sapi_shutdown();
      ext_php_rs_sapi_shutdown();
    }
  }
}

pub(crate) static SAPI_INIT: OnceCell<RwLock<Weak<Sapi>>> = OnceCell::new();

pub fn ensure_sapi() -> Result<Arc<Sapi>, EmbedStartError> {
  let weak_sapi = SAPI_INIT.get_or_try_init(|| Ok(RwLock::new(Weak::new())))?;

  if let Some(sapi) = weak_sapi
    .read()
    .map_err(|_| EmbedStartError::SapiNotInitialized)?
    .upgrade()
  {
    return Ok(sapi);
  }

  let mut rwlock = weak_sapi
    .write()
    .map_err(|_| EmbedStartError::SapiNotInitialized)?;

  let sapi = Sapi::new().map(Arc::new)?;
  *rwlock = Arc::downgrade(&sapi);

  Ok(sapi)
}

fn get_sapi() -> Result<Arc<Sapi>, EmbedRequestError> {
  if let Some(sapi) = SAPI_INIT.get() {
    if let Ok(read) = sapi.read() {
      if let Some(upgraded) = read.upgrade() {
        return Ok(upgraded);
      }
    }
  }

  Err(EmbedRequestError::SapiNotStarted)
}

//
// Sapi functions
//

// error_reporting =
//   E_ERROR | E_WARNING | E_PARSE | E_CORE_ERROR | E_CORE_WARNING |
//   E_COMPILE_ERROR | E_COMPILE_WARNING | E_RECOVERABLE_ERROR
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
  let mut globals = SapiGlobals::get_mut();

  for i in 0..globals.request_info.argc {
    maybe_efree(unsafe { *globals.request_info.argv.offset(i as isize) }.cast::<u8>());
  }

  globals.request_info.argc = 0;
  globals.request_info.argv = std::ptr::null_mut();

  maybe_efree(globals.request_info.request_method as *mut u8);
  maybe_efree(globals.request_info.content_type as *mut u8);
  maybe_efree(globals.request_info.query_string.cast::<u8>());
  maybe_efree(globals.request_info.request_uri.cast::<u8>());
  maybe_efree(globals.request_info.path_translated.cast::<u8>());
  maybe_efree(globals.request_info.auth_user.cast::<u8>());
  maybe_efree(globals.request_info.auth_password.cast::<u8>());
  maybe_efree(globals.request_info.auth_digest.cast::<u8>());

  maybe_efree(globals.request_info.cookie_data.cast::<u8>());

  ZEND_RESULT_CODE_SUCCESS
}

fn maybe_efree(ptr: *mut u8) {
  if !ptr.is_null() {
    unsafe {
      efree(ptr);
    }
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_ub_write(str: *const c_char, str_length: usize) -> usize {
  if str.is_null() || str_length == 0 {
    return 0;
  }

  let bytes = unsafe { std::slice::from_raw_parts(str as *const u8, str_length) };

  let len = bytes.len();
  if let Some(ctx) = RequestContext::current() {
    // Use new method name for clarity (FIXME.md #4)
    ctx.write_response_body(bytes);
  }
  len
}

#[no_mangle]
pub extern "C" fn sapi_module_flush(_server_context: *mut c_void) {
  unsafe { sapi_send_headers() };
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
      // Use new method that doesn't require mem::replace hacks (FIXME.md #4)
      ctx.add_response_header(name, value);
    }
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_read_post(buffer: *mut c_char, length: usize) -> usize {
  if length == 0 {
    return 0;
  }

  // Fixed body reading bug from FIXME.md #2
  // Now we properly consume from the mutable body instead of cloning
  RequestContext::current()
    .map(|ctx| {
      let body = ctx.request_body_mut();
      let actual_length = length.min(body.len());
      if actual_length == 0 {
        return 0;
      }

      // Properly consume from the original body buffer
      let chunk = body.split_to(actual_length);

      unsafe {
        std::ptr::copy_nonoverlapping(chunk.as_ptr() as *mut c_char, buffer, actual_length);
      }
      actual_length
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn sapi_module_read_cookies() -> *mut c_char {
  RequestContext::current()
    .map(|ctx| match ctx.request_parts().headers.get("Cookie") {
      Some(cookie) => estrdup(cookie.to_str().unwrap_or("")),
      None => std::ptr::null_mut(),
    })
    .unwrap_or(std::ptr::null_mut())
}

fn env_var<K, V>(
  vars: *mut ext_php_rs::types::Zval,
  key: K,
  value: V,
) -> Result<(), EmbedRequestError>
where
  K: AsRef<str>,
  V: AsRef<str>,
{
  let c_value = estrdup(value.as_ref());
  env_var_c(vars, key, c_value)?;
  maybe_efree(c_value.cast::<u8>());

  Ok(())
}

fn env_var_c<K>(
  vars: *mut ext_php_rs::types::Zval,
  key: K,
  c_value: *const c_char,
) -> Result<(), EmbedRequestError>
where
  K: AsRef<str>,
{
  let c_key = estrdup(key.as_ref());
  unsafe {
    php_register_variable(c_key, c_value, vars);
  }
  maybe_efree(c_key.cast::<u8>());

  Ok(())
}

#[no_mangle]
pub extern "C" fn sapi_module_register_server_variables(vars: *mut ext_php_rs::types::Zval) {
  // use ext_php_rs::ffi::php_import_environment_variables;
  // if let Some(f) = php_import_environment_variables {
  //   f(vars);
  // }

  if let Some(ctx) = RequestContext::current() {
    let request_parts = ctx.request_parts();
    let headers = &request_parts.headers;

    // Hack to allow ? syntax for the following code.
    // At the moment any errors are just swallowed, but these could be
    // collected and reported somehow in the future.
    Ok::<(), EmbedRequestError>(())
      .and_then(|_| {
        for (key, value) in headers.iter() {
          let value_string = value.to_str().unwrap_or("").to_string();
          let upper = key.as_str().to_ascii_uppercase();
          let cgi_key = format!("HTTP_{}", upper.replace("-", "_"));
          env_var(vars, cgi_key, value_string)?;
        }

        let globals = SapiGlobals::get();
        let req_info = &globals.request_info;

        let docroot = ctx.docroot();
        let docroot_str = docroot.display().to_string();

        let script_filename = req_info.path_translated;
        let script_name = if !req_info.request_uri.is_null() {
          req_info.request_uri
        } else {
          std::ptr::null_mut()
        };

        env_var(vars, "REQUEST_SCHEME", request_parts.uri.scheme_str().unwrap_or("http"))?;
        env_var(vars, "CONTEXT_PREFIX", "")?;
        env_var(vars, "SERVER_ADMIN", "webmaster@localhost")?;
        env_var(vars, "GATEWAY_INTERFACE", "CGI/1.1")?;

        // Laravel seems to think "/register" should be "/index.php/register"?
        // env_var_c(vars, "PHP_SELF", script_name as *mut c_char)?;
        env_var(vars, "PHP_SELF", request_parts.uri.path())?;

        // TODO: is "/register", should be "/index.php"
        env_var(vars, "SCRIPT_NAME", request_parts.uri.path())?;
        // env_var_c(vars, "SCRIPT_NAME", script_name as *mut c_char)?;
        env_var_c(vars, "PATH_INFO", script_name as *mut c_char)?;
        env_var_c(vars, "SCRIPT_FILENAME", script_filename)?;
        env_var_c(vars, "PATH_TRANSLATED", script_filename)?;
        env_var(vars, "DOCUMENT_ROOT", docroot_str.clone())?;
        env_var(vars, "CONTEXT_DOCUMENT_ROOT", docroot_str)?;

        if let Ok(server_name) = hostname::get() {
          if let Some(server_name) = server_name.to_str() {
            env_var(vars, "SERVER_NAME", server_name)?;
          }
        }

        if !req_info.request_uri.is_null() {
          env_var_c(vars, "REQUEST_URI", req_info.request_uri)?;
        }

        env_var(vars, "SERVER_PROTOCOL", "HTTP/1.1")?;

        let sapi = get_sapi()?;
        if let Ok(inner_sapi) = sapi.0.read() {
          env_var_c(vars, "SERVER_SOFTWARE", inner_sapi.name)?;
        }

        if let Some(socket_info) = request_parts.extensions.get::<http_handler::SocketInfo>() {
          if let Some(local) = socket_info.local {
            env_var(vars, "SERVER_ADDR", local.ip().to_string())?;
            env_var(vars, "SERVER_PORT", local.port().to_string())?;
          }
          if let Some(remote) = socket_info.remote {
            env_var(vars, "REMOTE_ADDR", remote.ip().to_string())?;
            env_var(vars, "REMOTE_PORT", remote.port().to_string())?;
          }
        }

        if !req_info.request_method.is_null() {
          env_var_c(vars, "REQUEST_METHOD", req_info.request_method)?;
        }

        if !req_info.cookie_data.is_null() {
          env_var_c(vars, "HTTP_COOKIE", req_info.cookie_data)?;
        }

        if !req_info.query_string.is_null() {
          env_var_c(vars, "QUERY_STRING", req_info.query_string)?;
        }

        Ok(())
      })
      .ok();
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_log_message(message: *const c_char, _syslog_type_int: c_int) {
  let message = unsafe { CStr::from_ptr(message) };
  if let Some(ctx) = RequestContext::current() {
    // Use new method that uses extension system (FIXME.md #4)
    ctx.write_response_log(message.to_bytes());
  }
}

//
// PHP Module
//

#[php_function]
pub fn apache_request_headers() -> Result<HashMap<String, String>, String> {
  let mut headers = HashMap::new();

  let request_parts = RequestContext::current()
    .map(|ctx| ctx.request_parts())
    .ok_or("Request context unavailable")?;

  for (key, value) in request_parts.headers.iter() {
    headers.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
  }

  Ok(headers)
}

#[php_module]
pub fn module(module: ModuleBuilder<'_>) -> ModuleBuilder<'_> {
  module.function(wrap_function!(apache_request_headers))
}

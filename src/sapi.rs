use std::{
  collections::HashMap,
  env::current_exe,
  ffi::{c_char, c_int, c_void, CStr},
  sync::{Arc, RwLock, Weak},
};

use bytes::Buf;

use ext_php_rs::{
  alloc::{efree, estrdup},
  builders::SapiBuilder,
  embed::{ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup, SapiModule},
  // exception::register_error_observer,
  ffi::{
    php_module_shutdown, php_module_startup, php_register_variable, sapi_headers_struct,
    sapi_send_headers, sapi_shutdown, sapi_startup, ZEND_RESULT_CODE_SUCCESS,
  },
  prelude::*,
  zend::{SapiGlobals, SapiHeader},
};

use once_cell::sync::OnceCell;

use crate::{extensions::ResponseStream, EmbedRequestError, EmbedStartError, RequestContext};
use http_handler::extensions::{BodyBuffer, ResponseLog};
use http_handler::RequestExt;
use once_cell::sync::Lazy;

// Fallback runtime for sapi callbacks running in blocking context
static FALLBACK_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
  tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("Failed to create Tokio runtime")
});

pub(crate) fn fallback_handle() -> &'static tokio::runtime::Handle {
  FALLBACK_RUNTIME.handle()
}

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
      .send_headers_function(sapi_module_send_headers)
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
}

impl Drop for Sapi {
  fn drop(&mut self) {
    let sapi = &mut self.0.write().unwrap();
    if let Some(shutdown) = sapi.shutdown {
      unsafe {
        shutdown(sapi.as_mut());
      }
    }

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
  // CRITICAL: Clear server_context BEFORE php_module_shutdown
  // to prevent sapi_flush from accessing freed RequestContext
  unsafe {
    php_module_shutdown();
  }

  {
    let mut globals = SapiGlobals::get_mut();
    globals.server_context = std::ptr::null_mut();
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
  use tokio::io::AsyncWriteExt;

  if str.is_null() || str_length == 0 {
    return 0;
  }

  // Send headers if not already sent (implicit header send on first output)
  unsafe {
    let globals = SapiGlobals::get();
    if globals.headers_sent == 0 {
      sapi_send_headers();
    }
  }

  let bytes = unsafe { std::slice::from_raw_parts(str as *const u8, str_length) };
  let len = bytes.len();

  if let Some(ctx) = RequestContext::current() {
    // Get ResponseStream from extensions
    if let Some(response_stream) = ctx.extensions().get::<ResponseStream>() {
      // Clone body to avoid holding ctx reference
      let mut body = response_stream.0.clone();

      // Use block_on to write asynchronously
      let result = fallback_handle().block_on(async { body.write_all(bytes).await });

      match result {
        Ok(_) => return len,
        Err(_) => return 0, // Write error
      }
    }
  }

  len
}

#[no_mangle]
pub extern "C" fn sapi_module_flush(_server_context: *mut c_void) {
  unsafe { sapi_send_headers() };
}

#[no_mangle]
pub extern "C" fn sapi_module_send_header(_header: *mut SapiHeader, _server_context: *mut c_void) {
  // This is called by PHP for each header, but we don't need to track them here.
  // PHP maintains its own list in sapi_headers_struct, which we read in
  // sapi_module_send_headers() when headers are finalized.
}

#[no_mangle]
pub extern "C" fn sapi_module_send_headers(sapi_headers: *mut sapi_headers_struct) -> c_int {
  use ext_php_rs::ffi::sapi_get_default_content_type;
  use ext_php_rs::zend::SapiHeader;

  // Extract status and mimetype as owned types BEFORE leaving PHP thread
  if let Some(ctx) = RequestContext::current() {
    let (status, mimetype_owned) = {
      let h = SapiGlobals::get().sapi_headers;
      let mut mime = h.mimetype;
      if mime.is_null() {
        mime = unsafe { sapi_get_default_content_type() };
      }

      let mime_str = if !mime.is_null() {
        unsafe { std::ffi::CStr::from_ptr(mime) }
          .to_str()
          .unwrap_or("text/html")
          .to_owned()
      } else {
        "text/html".to_owned()
      };

      // Free the mimetype if it was allocated
      if !mime.is_null() && mime != h.mimetype {
        unsafe { efree(mime.cast::<u8>()) };
      }

      (h.http_response_code as u16, mime_str)
    };

    // Extract headers from sapi_headers_struct
    let mut custom_headers = Vec::new();
    if !sapi_headers.is_null() {
      let headers_list = unsafe { &(*sapi_headers).headers };

      for header in headers_list.iter::<SapiHeader>() {
        let name = header.name();
        if let Some(value) = header.value() {
          custom_headers.push((name.to_string(), value.to_string()));
        }
      }
    }

    // Collect logs from ResponseLog extension
    let logs = if let Some(log_ext) = ctx.extensions().get::<ResponseLog>() {
      bytes::Bytes::copy_from_slice(log_ext.as_bytes())
    } else {
      bytes::Bytes::new()
    };

    // Signal headers sent with owned data including custom headers and logs
    ctx.signal_headers_sent_with_data(status, mimetype_owned.clone(), custom_headers, logs);
  }

  1 // SAPI_HEADER_SENT_SUCCESSFULLY
}

#[no_mangle]
pub extern "C" fn sapi_module_read_post(buffer: *mut c_char, length: usize) -> usize {
  use tokio::io::AsyncReadExt;

  if length == 0 {
    return 0;
  }

  let result = RequestContext::current()
    .and_then(|ctx| {
      // Get or create BodyBuffer extension
      let buffer_len = ctx
        .extensions()
        .get::<BodyBuffer>()
        .map(|b| b.len())
        .unwrap_or(0);

      // If buffer is empty and we have a request stream, wait for data
      if buffer_len == 0 {
        // Check if we have a request stream
        if let Some(request_stream) = ctx.extensions().get::<crate::extensions::RequestStream>() {
          // Clone body to avoid holding ctx reference
          let mut body = request_stream.0.clone();

          // Keep reading chunks until we have enough data or hit EOF
          loop {
            // Check current buffer size
            let current_buffer_len = ctx
              .extensions()
              .get::<BodyBuffer>()
              .map(|b| b.len())
              .unwrap_or(0);

            // If we have enough data, stop reading
            if current_buffer_len >= length {
              break;
            }

            // Read a chunk from the stream using block_on
            let read_result = fallback_handle().block_on(async {
              let mut chunk = vec![0u8; 8192];
              match body.read(&mut chunk).await {
                Ok(0) => None, // EOF
                Ok(n) => {
                  chunk.truncate(n);
                  Some(bytes::Bytes::from(chunk))
                }
                Err(_) => None, // Error, treat as EOF
              }
            });

            // Append data if we got some
            if let Some(data) = read_result {
              if !data.is_empty() {
                // Get or insert BodyBuffer and append data
                if let Some(body_buf) = ctx.extensions_mut().get_mut::<BodyBuffer>() {
                  body_buf.append(&data);
                } else {
                  ctx.extensions_mut().insert(BodyBuffer::from_bytes(data));
                }
              }
            } else {
              // EOF reached
              break;
            }
          }
        }
      }

      // Now read from the buffer
      // We need to consume data from the buffer, so we'll convert to BytesMut, split, then convert back
      ctx
        .extensions_mut()
        .get_mut::<BodyBuffer>()
        .and_then(|body| {
          let actual_length = length.min(body.len());
          if actual_length == 0 {
            return None;
          }

          // Get the bytes as a slice and copy to output buffer
          let bytes = body.as_bytes();
          unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buffer, actual_length);
          }

          // Now we need to remove the consumed bytes - convert to BytesMut, split, convert back
          let mut bytes_mut = std::mem::replace(body, BodyBuffer::new()).into_bytes_mut();
          bytes_mut.advance(actual_length);
          *body = BodyBuffer::from_bytes(bytes_mut.freeze());

          Some(actual_length)
        })
    })
    .unwrap_or(0);

  result
}

#[no_mangle]
pub extern "C" fn sapi_module_read_cookies() -> *mut c_char {
  RequestContext::current()
    .map(|ctx| match ctx.headers().get("Cookie") {
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
    let headers = ctx.headers();
    let uri = ctx.uri();

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

        // Get docroot from DocumentRoot extension
        let docroot_str = ctx
          .document_root()
          .map(|dr| dr.path.display().to_string())
          .unwrap_or_else(|| ".".to_string());

        let script_filename = req_info.path_translated;
        let script_name = if !req_info.request_uri.is_null() {
          req_info.request_uri
        } else {
          std::ptr::null_mut()
        };

        env_var(vars, "REQUEST_SCHEME", uri.scheme_str().unwrap_or("http"))?;
        env_var(vars, "CONTEXT_PREFIX", "")?;
        env_var(vars, "SERVER_ADMIN", "webmaster@localhost")?;
        env_var(vars, "GATEWAY_INTERFACE", "CGI/1.1")?;

        // Laravel seems to think "/register" should be "/index.php/register"?
        // env_var_c(vars, "PHP_SELF", script_name as *mut c_char)?;
        env_var(vars, "PHP_SELF", uri.path())?;

        // TODO: is "/register", should be "/index.php"
        env_var(vars, "SCRIPT_NAME", uri.path())?;
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

        if let Some(socket_info) = ctx.extensions().get::<http_handler::SocketInfo>() {
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
    // Append to ResponseLog extension for both streaming and buffered modes
    // Note: PHP's SAPI adds the newline, so we just append the message
    if let Some(log_ext) = ctx.extensions_mut().get_mut::<ResponseLog>() {
      log_ext.append(message.to_bytes());
    }
  }
}

//
// PHP Module
//

#[php_function]
pub fn apache_request_headers() -> Result<HashMap<String, String>, String> {
  let mut headers = HashMap::new();

  let ctx = RequestContext::current().ok_or("Request context unavailable")?;

  for (key, value) in ctx.headers().iter() {
    headers.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
  }

  Ok(headers)
}

#[php_module]
pub fn module(module: ModuleBuilder<'_>) -> ModuleBuilder<'_> {
  module.function(wrap_function!(apache_request_headers))
}

use std::{
  collections::HashMap, env::Args, ffi::{c_char, c_int, c_void, CStr, CString, NulError}, ops::Deref, path::PathBuf, sync::{OnceLock, RwLock}
};

use bytes::{Buf, BufMut};

use ext_php_rs::{
  builders::{IniBuilder, SapiBuilder},
  embed::{ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup, SapiModule},
  error::Error,
  exception::register_error_observer,
  ffi::{
    php_execute_script, php_module_shutdown, php_module_startup, php_register_variable,
    php_request_shutdown, php_request_startup, sapi_header_struct, sapi_shutdown, sapi_startup,
    zend_eval_string_ex, zend_stream_init_filename, ZEND_RESULT_CODE_SUCCESS
  },
  prelude::*,
  types::{ZendHashTable, ZendStr},
  zend::{
    try_catch, try_catch_first, ExecutorGlobals, ProcessGlobals, SapiGlobals, SapiHeader,
    SapiHeaders,
  },
};

use lang_handler::{Handler, Request, Response, ResponseBuilder};
use libc::free;

struct memory_stream {
  ptr: *mut c_char,
  len: usize,
  available: usize,
}

#[no_mangle]
unsafe extern "C" fn memory_stream_reader(
  handle: *mut c_void,
  buf: *mut c_char,
  len: usize,
) -> isize {
  let stream = handle as *mut memory_stream;
  if stream.is_null() {
    return 0;
  }

  let stream = unsafe { &mut *stream };
  if stream.available == 0 {
    return 0;
  }

  let read_len = std::cmp::min(len, stream.available);
  unsafe {
    std::ptr::copy_nonoverlapping(stream.ptr as *const c_char, buf, read_len);
  }
  stream.available -= read_len;
  stream.ptr = unsafe { stream.ptr.add(read_len) };
  read_len as isize
}

#[no_mangle]
unsafe extern "C" fn memory_stream_fsizer(handle: *mut c_void) -> usize {
  let stream = handle as *mut memory_stream;
  if stream.is_null() {
    return 0;
  }

  let stream = unsafe { &mut *stream };
  stream.len - stream.available
}

#[no_mangle]
unsafe extern "C" fn memory_stream_closer(_handle: *mut c_void) {
  // Nothing to do. The memory stream lifetime is managed by Rust
}

// This is a helper to ensure that PHP is initialized and deinitialized at the
// appropriate times.
struct Sapi(Box<SapiModule>);

impl Sapi {
  pub fn new<S>(_argv: Vec<S>) -> Self
  where
    S: AsRef<str>,
  {
    // let argv: Vec<&str> = argv.iter().map(|s| s.as_ref()).collect();
    // let argc = argv.len() as i32;
    // let mut argv_ptrs = argv
    //     .iter()
    //     .map(|v| v.as_ptr() as *mut c_char)
    //     .collect::<Vec<*mut c_char>>();

    let sapi = SapiBuilder::new("php_lang_handler", "PHP Lang Handler")
      .startup_function(sapi_module_startup)
      // .shutdown_function(sapi_module_shutdown)
      // .activate_function(sapi_module_activate)
      .deactivate_function(sapi_module_deactivate)
      .ub_write_function(sapi_module_ub_write)
      .send_header_function(sapi_module_send_header)
      .read_post_function(sapi_module_read_post)
      .read_cookies_function(sapi_module_read_cookies)
      .register_server_variables_function(sapi_module_register_server_variables)
      .log_message_function(sapi_module_log_message)
      // .executable_location(args.get(0))
      .build()
      .expect("Failed to build SAPI module");

    let mut boxed = Box::new(sapi);

    unsafe {
      ext_php_rs_sapi_startup();

      sapi_startup(boxed.as_mut());
      php_module_startup(boxed.as_mut(), get_module());
    }

    // TODO: Should maybe capture this to store in EmbedException rather than
    // writing to the ResponseBuilder here. When php_execute_script fails it
    // should read that and could return an error or write it to the
    // ResponseBuilder there.
    register_error_observer(|_error_type, _file, _line, message| {
      RequestContext::current().map(|ctx| {
        let message_str = message.as_str()
          .expect("Failed to convert message to string");
        ctx.response_builder().exception(message_str);
      });
    });

    Sapi(boxed)
  }

  fn do_startup(&mut self) -> Result<(), String> {
    let sapi = self.0.as_mut();
    let startup = sapi.startup.expect("No startup function");
    if unsafe { startup(sapi) } != ZEND_RESULT_CODE_SUCCESS {
      return Err("Failed to start PHP SAPI".to_string());
    }
    Ok(())
  }

  pub fn startup() -> Result<(), String> {
    match SAPI_INIT.get() {
      None => Err("SAPI not initialized".to_string()),
      Some(rwlock) => match rwlock.write() {
        Err(_) => Err("Failed to lock SAPI instance".to_string()),
        Ok(mut sapi) => {
          sapi.do_startup()?;
          Ok(())
        }
      },
    }
  }
}

impl Drop for Sapi {
  fn drop(&mut self) {
    unsafe {
      php_module_shutdown();
      sapi_shutdown();

      ext_php_rs_sapi_shutdown();
    }
  }
}

static SAPI_INIT: OnceLock<RwLock<Sapi>> = OnceLock::new();

#[derive(Debug)]
pub enum EmbedException {
  SapiStartupError,
  RequestStartupError,
  InvalidCString(NulError),
  HeaderNotFound(String),
  ExecuteError,
  Exception(String),
  Bailout,
  ResponseError,
  IoError(std::io::Error)
}

impl std::fmt::Display for EmbedException {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EmbedException::SapiStartupError => write!(f, "SAPI startup error"),
      EmbedException::RequestStartupError => write!(f, "Request startup error"),
      EmbedException::InvalidCString(e) => write!(f, "CString conversion error: {}", e.to_string()),
      EmbedException::HeaderNotFound(header) => write!(f, "Header not found: {}", header),
      EmbedException::ExecuteError => write!(f, "Script execution error"),
      EmbedException::Exception(e) => write!(f, "Exception thrown: {}", e),
      EmbedException::Bailout => write!(f, "PHP bailout"),
      EmbedException::ResponseError => write!(f, "Error building response"),
      EmbedException::IoError(e) => write!(f, "IO error: {}", e),
    }
  }
}

/// Embed a PHP script into a Rust application to handle HTTP requests.
#[derive(Debug)]
pub struct Embed {
  code: String,
  filename: Option<String>,
}

// An embed instance may be constructed on the main thread and then shared
// across multiple threads in a thread pool.
unsafe impl Send for Embed {}
unsafe impl Sync for Embed {}

impl Embed {
  /// Creates a new `Embed` instance.
  ///
  /// # Examples
  ///
  /// ```
  /// use php::Embed;
  ///
  /// let embed = Embed::new("echo 'Hello, world!';", Some("example.php"));
  /// ```
  pub fn new<C, F>(code: C, filename: Option<F>) -> Self
  where
    C: Into<String>,
    F: Into<String>,
  {
    Embed::new_with_argv::<C, F, String>(code, filename, vec![])
  }

  /// Creates a new `Embed` instance with command-line arguments.
  ///
  /// # Examples
  ///
  /// ```
  /// use php::Embed;
  ///
  /// let args = std::env::args();
  /// let embed = Embed::new_with_args("echo $argv[1];", Some("example.php"), args);
  /// ```
  pub fn new_with_args<C, F>(code: C, filename: Option<F>, args: Args) -> Self
  where
    C: Into<String>,
    F: Into<String>,
  {
    Embed::new_with_argv(code, filename, args.collect())
  }

  /// Creates a new `Embed` instance with command-line arguments.
  ///
  /// # Examples
  ///
  /// ```
  /// use php::{Embed, Handler, Request, Response};
  ///
  /// let embed = Embed::new_with_argv("echo $_SERVER['argv'][0];", Some("example.php"), vec![
  ///   "Hello, world!"
  /// ]);
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com").expect("invalid url")
  ///   .build();
  ///
  /// // let response = embed.handle(request).unwrap();
  ///
  /// // assert_eq!(response.status(), 200);
  /// # // TODO: Uncomment when argv gets passed through correctly.
  /// # // assert_eq!(response.body(), "Hello, world!");
  /// ```
  pub fn new_with_argv<C, F, S>(code: C, filename: Option<F>, argv: Vec<S>) -> Self
  where
    C: Into<String>,
    F: Into<String>,
    S: AsRef<str> + std::fmt::Debug,
  {
    SAPI_INIT.get_or_init(|| RwLock::new(Sapi::new(argv)));

    Embed {
      code: code.into(),
      filename: filename.map(|v| v.into()),
    }
  }
}

impl Handler for Embed {
  type Error = EmbedException;

  /// Handles an HTTP request.
  ///
  /// # Examples
  ///
  /// ```
  /// use php::{Embed, Handler, Request, Response};
  ///
  /// let handler = Embed::new("echo 'Hello, world!';", Some("example.php"));
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com").expect("invalid url")
  ///   .build();
  ///
  /// let response = handler.handle(request).unwrap();
  ///
  /// //assert_eq!(response.status(), 200);
  /// //assert_eq!(response.body(), "Hello, world!");
  /// ```
  fn handle(&self, request: Request) -> Result<Response, Self::Error> {
    unsafe {
      ext_php_rs::embed::ext_php_rs_sapi_per_thread_init();
    }

    // Initialize the SAPI module
    Sapi::startup().map_err(|_| EmbedException::SapiStartupError)?;

    // Get code and filename to execute
    let code = cstr(self.code.clone())?;
    let cwd = maybe_current_dir()?;
    let script_name = default_cstr(
      "<unnamed>",
      self.filename.clone().map(|v| {
        cwd
          .join(v)
          .canonicalize()
          .unwrap_or(cwd)
          .display()
          .to_string()
      }),
    )?;

    // Extract request information
    let request_method = cstr(request.method())?;

    let url = request.url();
    let query_string = cstr(url.query().unwrap_or(""))?;
    let path_translated = script_name;

    let headers = request.headers();
    let content_type = nullable_cstr(headers.get("Content-Type"))?;
    let content_length = headers
      .get("Content-Length")
      .map(|v| v.parse::<i64>().unwrap_or(0))
      .unwrap_or(0);
    let cookie_data = nullable_cstr(headers.get("Cookie"))?;

    // Prepare memory stream of the code
    let mut file_handle = unsafe {
      use ext_php_rs::ffi::{_zend_file_handle__bindgen_ty_1, zend_file_handle, zend_stream};

      let mut mem_stream = memory_stream {
        ptr: code,
        len: self.code.len(),
        available: self.code.len(),
      };

      let stream = zend_stream {
        handle: &mut mem_stream as *mut _ as *mut c_void,
        isatty: 0,
        reader: Some(memory_stream_reader),
        fsizer: Some(memory_stream_fsizer),
        closer: Some(memory_stream_closer),
      };

      let mut file_handle = zend_file_handle {
        handle: _zend_file_handle__bindgen_ty_1 { stream },
        filename: std::ptr::null_mut(),
        opened_path: std::ptr::null_mut(),
        type_: 2, // ZEND_HANDLE_STREAM
        primary_script: true,
        in_list: false,
        buf: std::ptr::null_mut(),
        len: 0,
      };

      zend_stream_init_filename(&mut file_handle, script_name);
      file_handle.handle = _zend_file_handle__bindgen_ty_1 { stream };
      file_handle.opened_path = file_handle.filename;
      file_handle.type_ = 2; // ZEND_HANDLE_STREAM
      file_handle.primary_script = true;

      // TODO: Make a scope to do zend_destroy_file_handle at the end.

      file_handle
    };

    let response = try_catch_first(|| {
      RequestContext::for_request(request.clone());

      // Set server context
      {
        let mut globals = SapiGlobals::get_mut();
        globals.options |= ext_php_rs::ffi::SAPI_OPTION_NO_CHDIR as i32;

        // Reset state
        globals.request_info.proto_num = 110;
        globals.request_info.argc = 0;
        globals.request_info.argv = std::ptr::null_mut();
        globals.request_info.headers_read = false;
        globals.sapi_headers.http_response_code = 200;

        // Set request info from request
        globals.request_info.request_method = request_method;
        globals.request_info.query_string = query_string;
        globals.request_info.path_translated = path_translated.clone();
        globals.request_info.request_uri = path_translated;

        // TODO: Add auth fields

        globals.request_info.content_type = content_type;
        globals.request_info.content_length = content_length;
        globals.request_info.cookie_data = cookie_data;
      }

      let response = {
        let _request_scope = RequestScope::new()?;

        // Run script in its own try/catch so bailout doesn't skip request shutdown.
        try_catch(|| {
          if !unsafe { php_execute_script(&mut file_handle) } {
            // return Err(EmbedException::ExecuteError);
          }

          // if unsafe {
          //     zend_eval_string_ex(code, std::ptr::null_mut(), filename, false)
          // } != ZEND_RESULT_CODE_SUCCESS {
          //     return Err(EmbedException::ExecuteError);
          // }

          if let Some(err) = ExecutorGlobals::take_exception() {
            {
              let mut globals = SapiGlobals::get_mut();
              globals.sapi_headers.http_response_code = 500;
            }

            let ex = Error::Exception(err);

            RequestContext::current().map(|ctx| {
              ctx.response_builder().exception(ex.to_string());
            });

            // TODO: Should exceptions be raised or only captured on
            // the response builder?
            return Err(EmbedException::Exception(ex.to_string()));
          }

          Ok(())
        })
        .map_or_else(|_err| Err(EmbedException::Bailout), |res| res)?;
        // .map_err(|_| EmbedException::Bailout)?;

        {
          let (mimetype, http_response_code) = {
            let globals = SapiGlobals::get();
            (
              globals.sapi_headers.mimetype,
              globals.sapi_headers.http_response_code,
            )
          };

          let mime = if mimetype.is_null() {
            "text/plain"
          } else {
            unsafe { CStr::from_ptr(mimetype as *const c_char) }
              .to_str()
              .unwrap_or("text/plain")
          };

          RequestContext::current().map(|ctx| {
            ctx
              .response_builder()
              .status(http_response_code)
              .header("Content-Type", mime);
          });
        }

        RequestContext::current()
          .map(|ctx| ctx.response_builder().build())
          .ok_or(EmbedException::ResponseError)?
      };

      Ok(response)
    })
    // Convert CatchError to a PhpException
    .map_or_else(|_err| Err(EmbedException::Bailout), |res| res)?;

    Ok(response)
  }
}

struct RequestScope();

impl RequestScope {
  fn new() -> Result<Self, EmbedException> {
    if unsafe { php_request_startup() } != ZEND_RESULT_CODE_SUCCESS {
      return Err(EmbedException::RequestStartupError);
    }

    Ok(RequestScope())
  }
}

impl Drop for RequestScope {
  fn drop(&mut self) {
    unsafe {
      php_request_shutdown(0 as *mut c_void);
    }
  }
}

// The request context for the PHP SAPI.
#[derive(Debug)]
struct RequestContext {
  request: Request,
  response_builder: ResponseBuilder,
}

impl RequestContext {
  // Sets the current request context for the PHP SAPI.
  //
  // # Examples
  //
  // ```
  // use php::{Request, RequestContext};
  //
  // let request = Request::builder()
  //  .method("GET")
  //  .url("http://example.com")
  //  .build();
  //
  // let mut context = RequestContext::new(request);
  // context.make_current();
  //
  // assert_eq!(context.request().method(), "GET");
  // ```
  fn for_request(request: Request) {
    let context = Box::new(RequestContext {
      request,
      response_builder: ResponseBuilder::new(),
    });
    let mut globals = SapiGlobals::get_mut();
    globals.server_context = Box::into_raw(context) as *mut c_void;
  }

  // Retrieve a mutable reference to the request context
  //
  // # Examples
  //
  // ```
  // use php::{Request, RequestContext};
  //
  // let request = Request::builder()
  //   .method("GET")
  //   .url("http://example.com")
  //   .build();
  //
  // let mut context = RequestContext::new(request);
  //
  // SapiGlobals::get_mut().server_context =
  //   &mut context as *mut RequestContext as *mut c_void;
  //
  // let current_context = RequestContext::current();
  // assert_eq!(current_context.request().method(), "GET");
  // ```
  pub fn current<'a>() -> Option<&'a mut RequestContext> {
    let ptr = {
      let globals = SapiGlobals::get();
      globals.server_context as *mut RequestContext
    };
    if ptr.is_null() {
      return None;
    }

    Some(unsafe { &mut *(ptr as *mut RequestContext) })
  }

  pub fn reclaim() -> Option<Box<RequestContext>> {
    let ptr = {
      let mut globals = SapiGlobals::get_mut();
      std::mem::replace(&mut globals.server_context, std::ptr::null_mut())
    };
    if ptr.is_null() {
      return None;
    }
    Some(unsafe { Box::from_raw(ptr as *mut RequestContext) })
  }

  // Returns a reference to the request.
  //
  // # Examples
  //
  // ```
  // use php::{Request, RequestContext};
  //
  // let request = Request::builder()
  //   .method("GET")
  //   .url("http://example.com")
  //   .build();
  //
  // let context = RequestContext::new(request);
  //
  // assert_eq!(context.request().method(), "GET");
  // ```
  pub fn request(&self) -> &Request {
    &self.request
  }

  // Returns a mutable reference to the response builder.
  //
  // # Examples
  //
  // ```
  // use php::{Request, RequestContext};
  //
  // let request = Request::builder()
  //   .method("GET")
  //   .url("http://example.com")
  //   .build();
  //
  // let mut context = RequestContext::new(request);
  //
  // context.response_builder().status(200);
  // ```
  pub fn response_builder(&mut self) -> &mut ResponseBuilder {
    &mut self.response_builder
  }
}

//
// PHP SAPI Functions
//

static HARDCODED_INI: &str = "
    display_errors=1
    register_argc_argv=1
    log_errors=1
    implicit_flush=1
    memory_limit=128MB
    output_buffering=0
    enable_post_data_reading=1
";

#[no_mangle]
pub extern "C" fn sapi_module_startup(
  sapi_module: *mut SapiModule,
) -> ext_php_rs::ffi::zend_result {
  let mut ini_builder = IniBuilder::new();
  let config = HARDCODED_INI
    .lines()
    .map(str::trim)
    .collect::<Vec<_>>()
    .join("\n");

  ini_builder.prepend(config);

  let mut sapi = unsafe { *sapi_module };
  sapi.ini_entries = ini_builder.finish();
  // sapi.php_ini_ignore_cwd = 1;
  // sapi.phpinfo_as_text = 1;
  // sapi.php_ini_path_override = "";

  unsafe { php_module_startup(sapi_module, get_module()) }
}

#[no_mangle]
pub extern "C" fn sapi_module_deactivate() -> c_int {
  {
    let mut globals = SapiGlobals::get_mut();

    globals.server_context = std::ptr::null_mut();
    globals.request_info.argc = 0;
    globals.request_info.argv = std::ptr::null_mut();

    // drop_str(globals.request_info.request_method);
    // drop_str(globals.request_info.query_string);
    // drop_str(globals.request_info.request_uri);
    // drop_str(globals.request_info.path_translated);
    // drop_str(globals.request_info.content_type);
    // drop_str(globals.request_info.cookie_data);
    // drop_str(globals.request_info.php_self);
    // drop_str(globals.request_info.auth_user);
    // drop_str(globals.request_info.auth_password);
    // drop_str(globals.request_info.auth_digest);
  }

  // TODO: When _is_ it safe to reclaim the request context?
  // RequestContext::reclaim();

  return 0;
}

#[no_mangle]
pub extern "C" fn sapi_module_ub_write(str: *const i8, str_length: usize) -> usize {
  if str.is_null() || str_length == 0 {
    return 0;
  }
  let bytes = unsafe { std::slice::from_raw_parts(str as *const u8, str_length) };
  let len = bytes.len();
  RequestContext::current().map(|ctx| {
    ctx.response_builder().body_write(bytes);
  });
  len
}

// #[no_mangle]
// pub extern "C" fn sapi_module_flush(_server_context: *mut c_void) {
//     ext_php_rs::ffi::sapi_send_headers();
// }

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
    RequestContext::current().map(|ctx| {
      ctx.response_builder().header(name, value);
    });
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_read_post(buffer: *mut c_char, length: usize) -> usize {
  if length == 0 {
    return 0;
  }

  let server_context = SapiGlobals::get().server_context as *mut RequestContext;
  let request = unsafe { &mut (*server_context).request };
  let body = request.body();

  let length = length.min(body.len());
  if length == 0 {
    return 0;
  }

  let chunk = body.take(length);

  unsafe {
    std::ptr::copy_nonoverlapping(chunk.chunk().as_ptr() as *mut c_char, buffer, length);
  }
  length
}

#[no_mangle]
pub extern "C" fn sapi_module_read_cookies() -> *mut c_char {
  SapiGlobals::get().request_info.cookie_data
}

#[no_mangle]
pub extern "C" fn sapi_module_register_server_variables(vars: *mut ext_php_rs::types::Zval) {
  unsafe {
    if let Some(php_import_environment_variables) =
      ext_php_rs::ffi::php_import_environment_variables
    {
      php_import_environment_variables(vars);
    }

    let globals = SapiGlobals::get();
    let req_info = &globals.request_info;

    let script_name = c"".as_ptr();
    let script_filename = req_info.path_translated;

    php_register_variable(cstr("PHP_SELF").unwrap(), script_name, vars);
    php_register_variable(cstr("SCRIPT_NAME").unwrap(), script_name, vars);
    php_register_variable(cstr("SCRIPT_FILENAME").unwrap(), script_filename, vars);
    php_register_variable(cstr("PATH_TRANSLATED").unwrap(), script_filename, vars);
    php_register_variable(cstr("DOCUMENT_ROOT").unwrap(), c"".as_ptr(), vars);

    // TODO: This should pull from the _real_ headers
    php_register_variable(cstr("HTTP_HOST").unwrap(), c"localhost:3000".as_ptr(), vars);

    if !req_info.request_method.is_null() {
      php_register_variable(
        cstr("REQUEST_METHOD").unwrap(),
        req_info.request_method,
        vars,
      );
    }

    if !req_info.cookie_data.is_null() {
      php_register_variable(cstr("COOKIE").unwrap(), req_info.cookie_data, vars);
    }

    if !req_info.query_string.is_null() {
      php_register_variable(cstr("QUERY_STRING").unwrap(), req_info.query_string, vars);
    }

    if !req_info.request_uri.is_null() {
      php_register_variable(cstr("REQUEST_URI").unwrap(), req_info.request_uri, vars);
    }
  };
}

#[no_mangle]
pub extern "C" fn sapi_module_log_message(message: *const c_char, _syslog_type_int: c_int) {
  let server_context = SapiGlobals::get().server_context as *mut RequestContext;
  let response_builder = unsafe { &mut (*server_context).response_builder };

  let message = unsafe { CStr::from_ptr(message) };

  response_builder.log_write(message.to_bytes());
}

//
// PHP Module Functions
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

//
// CString helpers
//

fn default_cstr<S: Into<String>, V: Into<String>>(
  default: S,
  maybe: Option<V>,
) -> Result<*mut c_char, EmbedException> {
  cstr(match maybe {
    Some(v) => v.into(),
    None => default.into(),
  })
}

fn nullable_cstr<S: Into<String>>(maybe: Option<S>) -> Result<*mut c_char, EmbedException> {
  match maybe {
    Some(v) => cstr(v.into()),
    None => Ok(std::ptr::null_mut()),
  }
}

fn cstr<S: AsRef<str>>(s: S) -> Result<*mut c_char, EmbedException> {
  CString::new(s.as_ref())
    .map_err(EmbedException::InvalidCString)
    .map(|cstr| cstr.into_raw())
}

fn reclaim_str(ptr: *const i8) -> CString {
  unsafe { CString::from_raw(ptr as *mut c_char) }
}

fn drop_str(ptr: *const i8) {
  if ptr.is_null() {
    return;
  }
  drop(reclaim_str(ptr));
}

fn maybe_current_dir() -> Result<PathBuf, EmbedException> {
  std::env::current_dir()
    .unwrap_or(std::path::PathBuf::from("/"))
    .canonicalize()
    .map_err(EmbedException::IoError)
}

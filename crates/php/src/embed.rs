use std::{
  collections::HashMap,
  env::Args,
  ffi::{c_char, c_int, c_void, CStr, CString, NulError},
  ops::Deref,
  path::{Path, PathBuf, StripPrefixError},
  str::FromStr,
  sync::{OnceLock, RwLock},
};

use bytes::{Buf, BufMut};

use ext_php_rs::{
  builders::{IniBuilder, SapiBuilder},
  embed::{ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup, SapiModule},
  error::Error,
  exception::register_error_observer,
  ffi::{
    php_execute_script, php_module_shutdown, php_module_startup, php_register_variable,
    php_request_shutdown, php_request_startup, sapi_get_default_content_type, sapi_header_struct,
    sapi_send_headers, sapi_shutdown, sapi_startup, zend_eval_string_ex, zend_stream_init_filename,
    ZEND_RESULT_CODE_SUCCESS,
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

// This is a helper to ensure that PHP is initialized and deinitialized at the
// appropriate times.
struct Sapi(Box<SapiModule>);

impl Sapi {
  pub fn new<S>(argv: Vec<S>) -> Self
  where
    S: AsRef<str>,
  {
    let argv: Vec<&str> = argv.iter().map(|s| s.as_ref()).collect();
    // let argc = argv.len() as i32;
    // let mut argv_ptrs = argv
    //     .iter()
    //     .map(|v| v.as_ptr() as *mut c_char)
    //     .collect::<Vec<*mut c_char>>();

    let mut sapi = SapiBuilder::new("php_lang_handler", "PHP Lang Handler")
      .startup_function(sapi_module_startup)
      // .shutdown_function(sapi_module_shutdown)
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
      .expect("Failed to build SAPI module");

    sapi.ini_defaults = Some(sapi_cli_ini_defaults);
    sapi.php_ini_path_override = std::ptr::null_mut();
    sapi.php_ini_ignore_cwd = 1;
    sapi.additional_functions = std::ptr::null();
    // sapi.phpinfo_as_text = 1;

    let exe_loc = argv.get(0).expect("should have exe location");
    let exe_loc = CString::from_str(exe_loc).expect("should construct exe location cstring");
    sapi.executable_location = exe_loc.as_ptr() as *mut i8;
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
        let message_str = message
          .as_str()
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
  InvalidStr(std::str::Utf8Error),
  HeaderNotFound(String),
  ExecuteError,
  Exception(String),
  Bailout,
  ResponseError,
  IoError(std::io::Error),
  RelativizeError(StripPrefixError),
  CanonicalizeError(std::io::Error),
}

impl std::fmt::Display for EmbedException {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EmbedException::SapiStartupError => write!(f, "SAPI startup error"),
      EmbedException::RequestStartupError => write!(f, "Request startup error"),
      EmbedException::InvalidCString(e) => write!(f, "CString conversion error: {}", e.to_string()),
      EmbedException::InvalidStr(e) => write!(f, "String conversion error: {}", e),
      EmbedException::HeaderNotFound(header) => write!(f, "Header not found: {}", header),
      EmbedException::ExecuteError => write!(f, "Script execution error"),
      EmbedException::Exception(e) => write!(f, "Exception thrown: {}", e),
      EmbedException::Bailout => write!(f, "PHP bailout"),
      EmbedException::ResponseError => write!(f, "Error building response"),
      EmbedException::IoError(e) => write!(f, "IO error: {}", e),
      EmbedException::RelativizeError(e) => write!(f, "Path relativization error: {}", e),
      EmbedException::CanonicalizeError(e) => write!(f, "Path canonicalization error: {}", e)
    }
  }
}

/// Embed a PHP script into a Rust application to handle HTTP requests.
#[derive(Debug)]
pub struct Embed {
  docroot: PathBuf
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
  pub fn new<C: AsRef<Path>>(docroot: C) -> Self {
    Embed::new_with_argv::<C, String>(docroot, vec![])
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
  pub fn new_with_args<C: AsRef<Path>>(docroot: C, args: Args) -> Self {
    Embed::new_with_argv(docroot, args.collect())
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
  pub fn new_with_argv<C, S>(docroot: C, argv: Vec<S>) -> Self
  where
    C: AsRef<Path>,
    S: AsRef<str> + std::fmt::Debug,
  {
    SAPI_INIT.get_or_init(|| RwLock::new(Sapi::new(argv)));

    let docroot = docroot
      .as_ref()
      .canonicalize()
      .expect("should exist");

    Embed {
      docroot
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

    let url = request.url();

    // Get code and filename to execute
    let request_uri = url.path();
    let path_translated = cstr(
      translate_path(&self.docroot, request_uri)?
        .display()
        .to_string()
    )?;
    let request_uri = cstr(request_uri)?;

    // Extract request information
    let request_method = cstr(request.method())?;
    let query_string = cstr(url.query().unwrap_or(""))?;

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

      let mut file_handle = zend_file_handle {
        handle: _zend_file_handle__bindgen_ty_1 { fp: std::ptr::null_mut() },
        filename: std::ptr::null_mut(),
        opened_path: std::ptr::null_mut(),
        type_: 0, //ZEND_HANDLE_FP
        primary_script: false,
        in_list: false,
        buf: std::ptr::null_mut(),
        len: 0,
      };

      zend_stream_init_filename(&mut file_handle, path_translated);

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
        globals.request_info.path_translated = path_translated;
        globals.request_info.request_uri = request_uri;

        // TODO: Add auth fields

        globals.request_info.content_type = content_type;
        globals.request_info.content_length = content_length;
        globals.request_info.cookie_data = cookie_data;
      }

      let response_builder = {
        let _request_scope = RequestScope::new()?;

        // Run script in its own try/catch so bailout doesn't skip request shutdown.
        try_catch(|| {
          if !unsafe { php_execute_script(&mut file_handle) } {
            // return Err(EmbedException::ExecuteError);
          }

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

        let (mimetype, http_response_code) = {
          let globals = SapiGlobals::get();
          (
            globals.sapi_headers.mimetype,
            globals.sapi_headers.http_response_code,
          )
        };

        let default_mime = str_from_cstr(unsafe { sapi_get_default_content_type() })?;

        let mime = if mimetype.is_null() {
          default_mime
        } else {
          str_from_cstr(mimetype as *const c_char).unwrap_or(default_mime)
        };

        RequestContext::current()
          .map(|ctx| {
            ctx
              .response_builder()
              .status(http_response_code)
              .header("Content-Type", mime)
          })
          .ok_or(EmbedException::ResponseError)?
      };

      Ok(response_builder.build())
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
pub extern "C" fn sapi_cli_ini_defaults(configuration_hash: *mut ext_php_rs::types::ZendHashTable) {
  let hash = unsafe { &mut *configuration_hash };

  let config = str::trim(HARDCODED_INI).lines().map(str::trim);

  for line in config {
    let mut parts = line.splitn(2, '=');
    let key = parts.next().unwrap();
    let value = parts.next().unwrap();
    hash.insert(key, value).unwrap();
  }
}

#[no_mangle]
pub extern "C" fn sapi_module_startup(
  sapi_module: *mut SapiModule,
) -> ext_php_rs::ffi::zend_result {
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

#[no_mangle]
pub extern "C" fn sapi_module_flush(_server_context: *mut c_void) {
  RequestContext::current().map(|ctx| {
    unsafe { sapi_send_headers() };
    let mut globals = SapiGlobals::get_mut();
    globals.headers_sent = 1;
    ctx
      .response_builder()
      .status(globals.sapi_headers.http_response_code);
  });
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

  let body = RequestContext::current()
    .map(|ctx| ctx.request().body())
    .unwrap();

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
    let globals = SapiGlobals::get();
    let req_info = &globals.request_info;

    let cwd = maybe_current_dir().unwrap();
    let cwd_cstr = cstr(cwd.as_os_str().to_str().unwrap()).unwrap();

    let script_filename = req_info.path_translated;
    let script_name = if !req_info.request_uri.is_null() {
      req_info.request_uri
    } else {
      c"".as_ptr()
    };

    php_register_variable(cstr("HTTP_SEC_FETCH_DEST").unwrap(), cstr("document").unwrap(), vars);
    php_register_variable(cstr("HTTP_USER_AGENT").unwrap(), cstr("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.4 Safari/605.1.15").unwrap(), vars);
    php_register_variable(cstr("HTTP_UPGRADE_INSECURE_REQUESTS").unwrap(), cstr("1").unwrap(), vars);
    php_register_variable(cstr("HTTP_ACCEPT").unwrap(), cstr("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8").unwrap(), vars);
    php_register_variable(cstr("HTTP_SEC_FETCH_SITE").unwrap(), cstr("none").unwrap(), vars);
    php_register_variable(cstr("HTTP_SEC_FETCH_MODE").unwrap(), cstr("navigate").unwrap(), vars);
    php_register_variable(cstr("HTTP_ACCEPT_LANGUAGE").unwrap(), cstr("en-CA,en-US;q=0.9,en;q=0.8").unwrap(), vars);
    php_register_variable(cstr("HTTP_PRIORITY").unwrap(), cstr("u=0, i").unwrap(), vars);
    php_register_variable(cstr("HTTP_ACCEPT_ENCODING").unwrap(), cstr("gzip, deflate").unwrap(), vars);
    php_register_variable(cstr("HTTP_CONNECTION").unwrap(), cstr("keep-alive").unwrap(), vars);
    php_register_variable(cstr("PATH").unwrap(), cstr("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin").unwrap(), vars);
    php_register_variable(cstr("SERVER_SIGNATURE").unwrap(), cstr("
      Apache/2.4.62 (Debian) Server at localhost Port 8080

").unwrap(), vars);
    php_register_variable(cstr("REQUEST_SCHEME").unwrap(), cstr("http").unwrap(), vars);
    php_register_variable(cstr("CONTEXT_PREFIX").unwrap(), cstr("").unwrap(), vars);
    php_register_variable(cstr("CONTEXT_DOCUMENT_ROOT").unwrap(), cwd_cstr, vars);
    php_register_variable(cstr("SERVER_ADMIN").unwrap(), cstr("webmaster@localhost").unwrap(), vars);
    php_register_variable(cstr("GATEWAY_INTERFACE").unwrap(), cstr("CGI/1.1").unwrap(), vars);

    php_register_variable(cstr("PHP_SELF").unwrap(), script_name, vars);
    php_register_variable(cstr("SCRIPT_NAME").unwrap(), script_name, vars);
    php_register_variable(cstr("SCRIPT_FILENAME").unwrap(), script_filename, vars);
    php_register_variable(cstr("PATH_TRANSLATED").unwrap(), script_filename, vars);
    php_register_variable(cstr("DOCUMENT_ROOT").unwrap(), cwd_cstr, vars);

    if !req_info.request_uri.is_null() {
      php_register_variable(cstr("REQUEST_URI").unwrap(), req_info.request_uri, vars);
    }

    php_register_variable(
      cstr("SERVER_PROTOCOL").unwrap(),
      cstr("HTTP/1.1").unwrap(),
      vars,
    );

    let sapi = SAPI_INIT.get().unwrap();
    php_register_variable(
      cstr("SERVER_SOFTWARE").unwrap(),
      sapi.read().expect("should read sapi").0.name,
      vars,
    );

    // TODO: REMOTE_ADDR, REMOTE_PORT

    // TODO: This should pull from the _real_ headers
    php_register_variable(cstr("HTTP_HOST").unwrap(), c"localhost:3000".as_ptr(), vars);
    php_register_variable(cstr("SERVER_NAME").unwrap(), c"localhost".as_ptr(), vars);
    php_register_variable(cstr("SERVER_ADDR").unwrap(), c"172.19.0.2".as_ptr(), vars);
    php_register_variable(cstr("SERVER_PORT").unwrap(), c"3000".as_ptr(), vars);
    php_register_variable(cstr("REMOTE_ADDR").unwrap(), c"192.168.65.1".as_ptr(), vars);
    php_register_variable(cstr("REMOTE_PORT").unwrap(), c"21845".as_ptr(), vars);

    if !req_info.request_method.is_null() {
      php_register_variable(
        cstr("REQUEST_METHOD").unwrap(),
        req_info.request_method,
        vars,
      );
    }

    if !req_info.cookie_data.is_null() {
      php_register_variable(cstr("HTTP_COOKIE").unwrap(), req_info.cookie_data, vars);
    }

    if !req_info.query_string.is_null() {
      php_register_variable(cstr("QUERY_STRING").unwrap(), req_info.query_string, vars);
    }
  };
}

#[no_mangle]
pub extern "C" fn sapi_module_log_message(message: *const c_char, _syslog_type_int: c_int) {
  let message = unsafe { CStr::from_ptr(message) };
  RequestContext::current().map(|ctx| {
    ctx.response_builder().log_write(message.to_bytes());
  });
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

fn str_from_cstr<'a>(ptr: *const c_char) -> Result<&'a str, EmbedException> {
  unsafe { CStr::from_ptr(ptr) }
    .to_str()
    .map_err(EmbedException::InvalidStr)
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

fn translate_path<D, P>(docroot: D, request_uri: P) -> Result<PathBuf, EmbedException>
where
  D: AsRef<Path>,
  P: AsRef<Path>,
{
  let docroot = docroot.as_ref().to_path_buf();
  let request_uri = request_uri.as_ref();
  let relative_uri = request_uri
    .strip_prefix("/")
    .map_err(EmbedException::RelativizeError)?;

  match docroot.join(relative_uri).join("index.php").canonicalize() {
    Ok(path) => Ok(path),
    Err(_) => {
      docroot
        .join(relative_uri)
        .canonicalize()
        .map_err(EmbedException::CanonicalizeError)
    }
  }
}

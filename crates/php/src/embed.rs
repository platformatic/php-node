use std::{
    collections::HashMap,
    env::Args,
    ffi::{c_char, c_int, c_void, CStr, CString},
    sync::OnceLock
};

use bytes::{Buf, BufMut};

use ext_php_rs::{
    builders::{IniBuilder, SapiBuilder},
    embed::{ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup, SapiModule},
    ffi::{
        php_module_shutdown, php_module_startup, php_request_shutdown, php_request_startup, sapi_header_struct, sapi_shutdown, sapi_startup, zend_eval_string_ex, ZEND_RESULT_CODE_SUCCESS
    },
    prelude::*,
    zend::{try_catch_first, ExecutorGlobals, SapiGlobals, SapiHeader, SapiHeaders}
};

use lang_handler::{Handler, Request, Response, ResponseBuilder};

// This is a helper to ensure that PHP is initialized and deinitialized at the
// appropriate times.
struct PhpInit;

impl PhpInit {
    pub fn new<S>(_argv: Vec<S>) -> Self
    where
        S: AsRef<str>
    {
        // let argv: Vec<&str> = argv.iter().map(|s| s.as_ref()).collect();
        // let argc = argv.len() as i32;
        // let mut argv_ptrs = argv
        //     .iter()
        //     .map(|v| v.as_ptr() as *mut c_char)
        //     .collect::<Vec<*mut c_char>>();

        unsafe {
            ext_php_rs_sapi_startup();
        }
        PhpInit
    }
}

impl Drop for PhpInit {
    fn drop(&mut self) {
        unsafe {
            ext_php_rs_sapi_shutdown();
        }
    }
}

static PHP_INIT: OnceLock<PhpInit> = OnceLock::new();

/// Embed a PHP script into a Rust application to handle HTTP requests.
#[derive(Debug, Clone)]
pub struct Embed {
    code: String,
    filename: Option<String>,
    sapi: SapiModule
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
        F: Into<String>
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
        F: Into<String>
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
        let sapi = SapiBuilder::new("php_lang_handler", "PHP Lang Handler")
            .startup_function(sapi_module_startup)
            // .shutdown_function(sapi_module_shutdown)
            // .activate_function(sapi_module_activate)
            .deactivate_function(sapi_module_deactivate)
            .ub_write_function(sapi_module_ub_write)
            .send_header_function(sapi_module_send_header)
            .read_post_function(sapi_module_read_post)
            .read_cookies_function(sapi_module_read_cookies)
            // .register_server_variables_function(sapi_module_register_server_variables)
            .log_message_function(sapi_module_log_message)
            // .executable_location(args.get(0))
            .build()
            .expect("Failed to build SAPI module");

        PHP_INIT.get_or_init(|| PhpInit::new(argv));

        unsafe {
            sapi_startup(sapi.into_raw());
            php_module_startup(sapi.into_raw(), get_module());
        }

        Embed {
            code: code.into(),
            filename: filename.map(|v| v.into()),
            sapi
        }
    }
}

impl Drop for Embed {
    fn drop(&mut self) {
        unsafe {
            php_module_shutdown();
            sapi_shutdown();
        }
    }
}

impl Handler for Embed {
    type Error = String;

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
        let startup = self.sapi.startup.expect("No startup function");
        let result = unsafe {
            startup(self.sapi.into_raw())
        };
        if result != ZEND_RESULT_CODE_SUCCESS {
            return Err("Failed to start PHP SAPI".to_string());
        }

        let mut request_context = try_catch_first(|| {
            let code = CString::new(self.code.clone())
                .unwrap();

            let filename = CString::new(self.filename.clone().unwrap_or("<unnamed>".to_string()))
                .unwrap();

            let request_method = CString::new(request.method()).unwrap();
            let url = request.url();
            let query_string = CString::new(url.query().unwrap_or("")).unwrap();
            let path_translated = CString::new(url.path()).unwrap();

            println!("request_method: {:?}", request_method);

            let mut request_context = RequestContext::new(request.clone());

            // Set server context
            {
                let mut globals = SapiGlobals::get_mut();
                globals.server_context = &mut request_context as *mut _ as *mut c_void;

                globals.options |= ext_php_rs::ffi::SAPI_OPTION_NO_CHDIR as i32;

                // Reset state
                globals.request_info.argc = 0;
                globals.request_info.argv = std::ptr::null_mut();
                globals.sapi_headers.http_response_code = 200;

                // Set request info from request
                globals.request_info.request_method = request_method.into_raw();
                globals.request_info.query_string = query_string.into_raw();
                globals.request_info.path_translated = path_translated.clone().into_raw();
                globals.request_info.request_uri = path_translated.into_raw();

                // TODO: Add auth fields

                let headers = request.headers();
                if let Some(content_type) = headers.get("Content-Type") {
                    globals.request_info.content_type = CString::new(content_type)
                        .expect("Failed to set content type to request info")
                        .into_raw();
                }
                if let Some(content_length) = headers.get("Content-Length") {
                    globals.request_info.content_length = content_length.parse()
                        .expect("Failed to parse content length");
                }
                if let Some(cookie) = headers.get("Cookie") {
                    globals.request_info.cookie_data = CString::new(cookie)
                        .expect("Failed to set cookie data to request info")
                        .into_raw();
                }
            }

            if unsafe { php_request_startup() } != ZEND_RESULT_CODE_SUCCESS {
                return Err::<RequestContext, PhpException>(
                    PhpException::default("Failed to start PHP request".to_string())
                );
            }

            {
                let mut globals = SapiGlobals::get_mut();
                globals.request_info.proto_num = 110;
            }

            let eval_result = unsafe {
                zend_eval_string_ex(code.into_raw(), std::ptr::null_mut(), filename.into_raw(), false)
            };
            if eval_result != ZEND_RESULT_CODE_SUCCESS {
                return Err::<RequestContext, PhpException>(
                    PhpException::default("Failed to evaluate PHP code".to_string())
                );
            }

            if let Some(_err) = ExecutorGlobals::take_exception() {
                let mut globals = SapiGlobals::get_mut();
                globals.sapi_headers.http_response_code = 500;
                // TODO: Figure out how to read exception messages.
                // request_context.response_builder().exception(err);

                let mut eg = ExecutorGlobals::get_mut();
                eg.exception = std::ptr::null_mut();
                eg.exit_status = 1;

                return Err::<RequestContext, PhpException>(
                    PhpException::default("something went wrong".to_string())
                );
            }

            {
                let mut globals = SapiGlobals::get_mut();
                globals.sapi_headers.http_response_code = 200;

                let mime = if globals.sapi_headers.mimetype.is_null() {
                    "text/plain"
                } else {
                    unsafe { CStr::from_ptr(globals.sapi_headers.mimetype as *const c_char) }
                        .to_str()
                        .unwrap_or("text/plain")
                };

                request_context.response_builder().header("Content-Type", mime);
                request_context.response_builder().status(globals.sapi_headers.http_response_code);
            }

            unsafe {
                php_request_shutdown(0 as *mut c_void);
            }

            Ok::<RequestContext, PhpException>(request_context)
        })
            .expect("Failed to execute PHP script")
            .expect("Failed to handle request");

        Ok(request_context.response_builder().build())
    }
}

// The request context for the PHP SAPI.
struct RequestContext {
    request: Request,
    response_builder: ResponseBuilder
}

impl RequestContext {
    // Creates a new request context to handle an HTTP request within the PHP SAPI.
    //
    // # Arguments
    //
    // * `request` - The HTTP request to handle.
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
    // ```
    fn new(request: Request) -> Self {
        RequestContext {
            request,
            response_builder: ResponseBuilder::new()
        }
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

// Retrieve a mutable reference to the request context
fn get_mut_context<'a>() -> &'a mut RequestContext {
    let globals = SapiGlobals::get_mut();
    unsafe {
        &mut *(globals.server_context as *mut RequestContext)
    }
}

// Reclaim a string given to the SAPI
fn reclaim_str(ptr: *const i8) -> CString {
    unsafe {
        CString::from_raw(ptr as *mut c_char)
    }
}

// Drop a string
fn drop_str(ptr: *const i8) {
    if ptr.is_null() {
        return;
    }
    drop(reclaim_str(ptr));
}

//
// PHP SAPI Functions
//

static HARDCODED_INI: &str = "
    display_errors=0
    register_argc_argv=1
    log_errors=1
    implicit_flush=1
    memory_limit=128MB
    output_buffering=0
";

#[no_mangle]
pub extern "C" fn sapi_module_startup(sapi_module: *mut SapiModule) -> ext_php_rs::ffi::zend_result {
    let mut ini_builder = IniBuilder::new();
    let config = HARDCODED_INI
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");

    ini_builder.prepend(config);

    unsafe { *sapi_module }.ini_entries = ini_builder.finish();

    let result = unsafe {
        php_module_startup(sapi_module, std::ptr::null_mut())
    };
    if result != ZEND_RESULT_CODE_SUCCESS {
        return result;
    }

    result
    // unsafe {
    //     php_module_startup(SapiModule::get_mut().into_raw(), std::ptr::null_mut())
    // }
}

#[no_mangle]
pub extern "C" fn sapi_module_deactivate() -> c_int {
    let mut globals = SapiGlobals::get_mut();

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
    // drop_str(globals.request_info.auth_user);
    // drop_str(globals.request_info.auth_password);
    // drop_str(globals.request_info.auth_digest);

    return 0;
}

#[no_mangle]
pub extern "C" fn sapi_module_ub_write(str: *const i8, str_length: usize) -> usize {
    let bytes = unsafe {
        std::slice::from_raw_parts(str as *const u8, str_length)
    };
    let len = bytes.len();
    get_mut_context().response_builder.body_write(bytes);
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
        get_mut_context().response_builder.header(name, value);
    }
}

#[no_mangle]
pub extern "C" fn sapi_module_read_post(buffer: *mut c_char, length: usize) -> usize {
    let server_context = SapiGlobals::get().server_context as *mut RequestContext;

    let request = unsafe { &mut (*server_context).request };
    let body = request.body();

    let length = length.min(body.len());
    let chunk = body.take(length);

    unsafe {
        std::ptr::copy_nonoverlapping(chunk.chunk().as_ptr() as *mut c_char, buffer, length);
    }
    length
}

#[no_mangle]
pub extern "C" fn sapi_module_read_cookies() -> *mut c_char {
    let request_info = SapiGlobals::get().request_info;
    let result = request_info.cookie_data;
    result
}

// #[no_mangle]
// pub extern "C" fn sapi_module_register_server_variables(vars: *mut ext_php_rs::zend::Zval) {
//     ext_php_rs::ffi::php_import_environment_variables();
// }

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
pub fn apache_request_headers() -> HashMap<String, String> {
    let mut headers = HashMap::new();

    get_mut_context().request().headers().iter().for_each(|(key, value)| {
        headers.insert(key.to_string(), value.into());
    });

    headers
}

#[php_module]
pub fn module(module: ModuleBuilder<'_>) -> ModuleBuilder<'_> {
    module.function(wrap_function!(apache_request_headers))
}

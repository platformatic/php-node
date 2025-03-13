use std::{
    env::Args,
    ffi::{c_void, c_char, CStr, CString}
};
use std::sync::OnceLock;

use lang_handler::{Handler, Request, Response};

use crate::sys;

// This is a helper to ensure that PHP is initialized and deinitialized at the
// appropriate times.
struct PhpInit;

impl PhpInit {
    pub fn new<S>(argv: Vec<S>) -> Self
    where
        S: AsRef<str>
    {
        let argv: Vec<&str> = argv.iter().map(|s| s.as_ref()).collect();
        let argc = argv.len() as i32;
        let mut argv_ptrs = argv
            .iter()
            .map(|v| v.as_ptr() as *mut c_char)
            .collect::<Vec<*mut c_char>>();

        unsafe {
            sys::php_http_init(argc, argv_ptrs.as_mut_ptr());
        }
        PhpInit
    }
}

impl Drop for PhpInit {
    fn drop(&mut self) {
        unsafe {
            sys::php_http_destruct();
        }
    }
}

static PHP_INIT: OnceLock<PhpInit> = OnceLock::new();

/// Embed a PHP script into a Rust application to handle HTTP requests.
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
/// assert_eq!(response.status(), 200);
/// assert_eq!(response.body(), "Hello, world!");
/// ```
#[derive(Debug, Clone)]
pub struct Embed {
    code: String,
    filename: Option<String>,
}

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
    /// let response = embed.handle(request).unwrap();
    ///
    /// assert_eq!(response.status(), 200);
    /// # // TODO: Uncomment when argv gets passed through correctly.
    /// # // assert_eq!(response.body(), "Hello, world!");
    /// ```
    pub fn new_with_argv<C, F, S>(code: C, filename: Option<F>, argv: Vec<S>) -> Self
    where
        C: Into<String>,
        F: Into<String>,
        S: AsRef<str> + std::fmt::Debug,
    {
        PHP_INIT.get_or_init(|| PhpInit::new(argv));

        Embed {
            code: code.into(),
            filename: filename.map(|v| v.into())
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
    /// assert_eq!(response.status(), 200);
    /// assert_eq!(response.body(), "Hello, world!");
    /// ```
    fn handle(&self, request: Request) -> Result<Response, Self::Error> {
        let code = CString::new(self.code.clone())
            .unwrap();

        let filename = self.filename
            .as_ref()
            .map(|v| CString::new(v.clone()))
            .unwrap_or(CString::new("<unnamed>"))
            .unwrap();

        let mut request: lang_handler::lh_request_t = request.into();
        let request = &mut request as *mut _ as *mut sys::lh_request_t;

        let response = unsafe {
            sys::php_http_handle_request(code.as_ptr(), filename.as_ptr(), request)
        };

        if response.is_null() {
            return Err("Failed to handle request".into());
        }

        let response = unsafe {
            &*(response as *mut lang_handler::lh_response_t)
        };

        Ok(response.into())
    }
}

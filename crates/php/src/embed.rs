use std::{
  env::Args,
  ffi::c_char,
  path::{Path, PathBuf},
};

use ext_php_rs::{
  error::Error,
  ffi::{
    _zend_file_handle__bindgen_ty_1, php_execute_script, sapi_get_default_content_type,
    zend_file_handle, zend_stream_init_filename,
  },
  zend::{try_catch, try_catch_first, ExecutorGlobals, SapiGlobals},
};

use lang_handler::{Handler, Request, Response};

use crate::{
  sapi::ensure_sapi,
  strings::{cstr, nullable_cstr, str_from_cstr, translate_path},
  EmbedException, RequestContext, RequestScope, Sapi,
};

/// Embed a PHP script into a Rust application to handle HTTP requests.
#[derive(Debug)]
pub struct Embed {
  docroot: PathBuf,
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
  /// use std::env::current_dir;
  /// use php::Embed;
  ///
  /// let docroot = current_dir()
  ///   .expect("should have current_dir");
  ///
  /// let embed = Embed::new(docroot);
  /// ```
  pub fn new<C: AsRef<Path>>(docroot: C) -> Result<Self, EmbedException> {
    Embed::new_with_argv::<C, String>(docroot, vec![])
  }

  /// Creates a new `Embed` instance with command-line arguments.
  ///
  /// # Examples
  ///
  /// ```
  /// use std::env::{args, current_dir};
  /// use php::Embed;
  ///
  /// let docroot = current_dir()
  ///   .expect("should have current_dir");
  ///
  /// let embed = Embed::new_with_args(docroot, args());
  /// ```
  pub fn new_with_args<C>(docroot: C, args: Args) -> Result<Self, EmbedException>
  where
    C: AsRef<Path>,
  {
    Embed::new_with_argv(docroot, args.collect())
  }

  /// Creates a new `Embed` instance with command-line arguments.
  ///
  /// # Examples
  ///
  /// ```
  /// use std::env::current_dir;
  /// use php::{Embed, Handler, Request, Response};
  ///
  /// let docroot = current_dir()
  ///   .expect("should have current_dir");
  ///
  /// let embed = Embed::new_with_argv(docroot, vec![
  ///   "foo"
  /// ]);
  /// ```
  pub fn new_with_argv<C, S>(docroot: C, argv: Vec<S>) -> Result<Self, EmbedException>
  where
    C: AsRef<Path>,
    S: AsRef<str> + std::fmt::Debug,
  {
    ensure_sapi(argv)?;

    let docroot = docroot
      .as_ref()
      .canonicalize()
      .map_err(|_| EmbedException::DocRootNotFound(docroot.as_ref().display().to_string()))?;

    Ok(Embed { docroot })
  }

  /// Get the docroot used for this Embed instance
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::env::current_dir;
  /// use php::Embed;
  ///
  ///
  /// let docroot = current_dir()
  ///   .expect("should have current_dir");
  ///
  /// let embed = Embed::new(&docroot)
  ///   .expect("should have constructed Embed");
  ///
  /// assert_eq!(embed.docroot(), docroot.as_path());
  /// ```
  pub fn docroot(&self) -> &Path {
    self.docroot.as_path()
  }
}

impl Handler for Embed {
  type Error = EmbedException;

  /// Handles an HTTP request.
  ///
  /// # Examples
  ///
  /// ```
  /// use std::env::current_dir;
  /// use php::{Embed, Handler, Request, Response};
  ///
  /// let docroot = current_dir()
  ///   .expect("should have current_dir");
  ///
  /// let handler = Embed::new(docroot)
  ///   .expect("should construct Embed");
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let response = handler.handle(request)
  ///   .expect("should handle request");
  ///
  /// //assert_eq!(response.status(), 200);
  /// //assert_eq!(response.body(), "Hello, world!");
  /// ```
  fn handle(&self, request: Request) -> Result<Response, Self::Error> {
    unsafe {
      ext_php_rs::embed::ext_php_rs_sapi_per_thread_init();
    }

    // Initialize the SAPI module
    Sapi::startup()?;

    let url = request.url();

    // Get code and filename to execute
    let request_uri = url.path();
    let path_translated = cstr(
      translate_path(&self.docroot, request_uri)?
        .display()
        .to_string(),
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
      let mut file_handle = zend_file_handle {
        handle: _zend_file_handle__bindgen_ty_1 {
          fp: std::ptr::null_mut(),
        },
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

            if let Some(ctx) = RequestContext::current() {
              ctx.response_builder().exception(ex.to_string());
            }

            // TODO: Should exceptions be raised or only captured on
            // the response builder?
            return Err(EmbedException::Exception(ex.to_string()));
          }

          Ok(())
        })
        .unwrap_or(Err(EmbedException::Bailout))?;

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
          .ok_or(EmbedException::ResponseBuildError)?
      };

      Ok(response_builder.build())
    })
    .unwrap_or(Err(EmbedException::Bailout))?;

    Ok(response)
  }
}

use std::{
  env::Args,
  ops::DerefMut,
  path::{Path, PathBuf},
  sync::Arc,
};

use ext_php_rs::{
  alloc::{efree, estrdup},
  error::Error,
  ffi::{php_execute_script, sapi_get_default_content_type},
  zend::{try_catch, try_catch_first, ExecutorGlobals, SapiGlobals},
};

use lang_handler::{rewrite::Rewriter, Handler, Request, Response};

use super::{
  sapi::{ensure_sapi, Sapi},
  scopes::{FileHandleScope, RequestScope},
  strings::translate_path,
  EmbedRequestError, EmbedStartError, RequestContext,
};

/// Embed a PHP script into a Rust application to handle HTTP requests.
pub struct Embed {
  docroot: PathBuf,
  args: Vec<String>,

  // NOTE: This needs to hold the SAPI to keep it alive
  #[allow(dead_code)]
  sapi: Arc<Sapi>,

  rewriter: Option<Box<dyn Rewriter>>,
}

impl std::fmt::Debug for Embed {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Embed")
      .field("docroot", &self.docroot)
      .field("args", &self.args)
      .field("sapi", &self.sapi)
      .field("rewriter", &"Box<dyn Rewriter>")
      .finish()
  }
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
  /// let embed = Embed::new(docroot, None);
  /// ```
  pub fn new<C>(docroot: C, rewriter: Option<Box<dyn Rewriter>>) -> Result<Self, EmbedStartError>
  where
    C: AsRef<Path>,
  {
    Embed::new_with_argv::<C, String>(docroot, rewriter, vec![])
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
  /// let embed = Embed::new_with_args(docroot, None, args());
  /// ```
  pub fn new_with_args<C>(
    docroot: C,
    rewriter: Option<Box<dyn Rewriter>>,
    args: Args,
  ) -> Result<Self, EmbedStartError>
  where
    C: AsRef<Path>,
  {
    Embed::new_with_argv(docroot, rewriter, args.collect())
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
  /// let embed = Embed::new_with_argv(docroot, None, vec![
  ///   "foo"
  /// ]);
  /// ```
  pub fn new_with_argv<C, S>(
    docroot: C,
    rewriter: Option<Box<dyn Rewriter>>,
    argv: Vec<S>,
  ) -> Result<Self, EmbedStartError>
  where
    C: AsRef<Path>,
    S: AsRef<str> + std::fmt::Debug,
  {
    let docroot_path = docroot.as_ref();
    let docroot = docroot_path
      .canonicalize()
      .map_err(|_| EmbedStartError::DocRootNotFound(docroot_path.display().to_string()))?;

    Ok(Embed {
      docroot,
      args: argv.iter().map(|v| v.as_ref().to_string()).collect(),
      sapi: ensure_sapi()?,
      rewriter,
    })
  }

  /// Get the docroot used for this Embed instance
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::env::current_dir;
  /// use php::Embed;
  ///
  /// let docroot = current_dir()
  ///   .expect("should have current_dir");
  ///
  /// let embed = Embed::new(&docroot, None)
  ///   .expect("should have constructed Embed");
  ///
  /// assert_eq!(embed.docroot(), docroot.as_path());
  /// ```
  pub fn docroot(&self) -> &Path {
    self.docroot.as_path()
  }
}

impl Handler for Embed {
  type Error = EmbedRequestError;

  /// Handles an HTTP request.
  ///
  /// # Examples
  ///
  /// ```
  /// use std::{env::temp_dir, fs::File, io::Write};
  /// use php::{Embed, Handler, Request, Response, MockRoot};
  ///
  /// let docroot = MockRoot::builder()
  ///   .file("index.php", "<?php echo \"Hello, World!\"; ?>")
  ///   .build()
  ///   .expect("should prepare docroot");
  ///
  /// let handler = Embed::new(docroot.clone(), None)
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
    let docroot = self.docroot.clone();

    // Initialize the SAPI module
    self.sapi.startup()?;

    // Get REQUEST_URI _first_ as it needs the pre-rewrite state.
    let url = request.url();
    let request_uri = url.path();

    // Apply request rewriting rules
    let mut request = request.clone();
    if let Some(rewriter) = &self.rewriter {
      request = rewriter
        .rewrite(request, &docroot)
        .map_err(EmbedRequestError::RequestRewriteError)?;
    }

    let translated_path = translate_path(&docroot, request.url().path())?
      .display()
      .to_string();

    // Convert REQUEST_URI and PATH_TRANSLATED to C strings
    let request_uri = estrdup(request_uri);
    let path_translated = estrdup(translated_path.clone());

    // Extract request method, query string, and headers
    let request_method = estrdup(request.method());
    let query_string = estrdup(url.query().unwrap_or(""));

    let headers = request.headers();
    let content_type = headers
      .get("Content-Type")
      .map(estrdup)
      .unwrap_or(std::ptr::null_mut());
    let content_length = headers
      .get("Content-Length")
      .map(|v| v.parse::<i64>().unwrap_or(0))
      .unwrap_or(0);

    // Prepare argv and argc
    let argc = self.args.len() as i32;
    let mut argv_ptrs = vec![];
    for arg in self.args.iter() {
      argv_ptrs.push(estrdup(arg.to_owned()));
    }

    let script_name = translated_path.clone();

    let response = try_catch_first(|| {
      RequestContext::for_request(request.clone(), docroot.clone());

      // Set server context
      {
        let mut globals = SapiGlobals::get_mut();
        globals.options |= ext_php_rs::ffi::SAPI_OPTION_NO_CHDIR as i32;

        // Reset state
        globals.request_info.proto_num = 110;
        globals.request_info.argc = argc;
        globals.request_info.argv = argv_ptrs.as_mut_ptr();
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
      }

      let response_builder = {
        let _request_scope = RequestScope::new()?;

        // Run script in its own try/catch so bailout doesn't skip request shutdown.
        {
          let mut file_handle = FileHandleScope::new(script_name.clone());
          try_catch(|| unsafe { php_execute_script(file_handle.deref_mut()) })
            .map_err(|_| EmbedRequestError::Bailout)?;
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

          return Err(EmbedRequestError::Exception(ex.to_string()));
        };

        let (mut mimetype, http_response_code) = {
          let h = SapiGlobals::get().sapi_headers;
          (h.mimetype, h.http_response_code)
        };

        if mimetype.is_null() {
          mimetype = unsafe { sapi_get_default_content_type() };
        }

        let mime_ptr =
          unsafe { mimetype.as_ref() }.ok_or(EmbedRequestError::FailedToDetermineContentType)?;

        let mime = unsafe { std::ffi::CStr::from_ptr(mime_ptr) }
          .to_str()
          .map_err(|_| EmbedRequestError::FailedToDetermineContentType)?
          .to_owned();

        unsafe {
          efree(mimetype.cast::<u8>());
        }

        RequestContext::current()
          .map(|ctx| {
            ctx
              .response_builder()
              .status(http_response_code)
              .header("Content-Type", mime)
          })
          .ok_or(EmbedRequestError::ResponseBuildError)?
      };

      Ok(response_builder.build())
    })
    .unwrap_or(Err(EmbedRequestError::Bailout))?;

    RequestContext::reclaim();

    Ok(response)
  }
}

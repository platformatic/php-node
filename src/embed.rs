use std::{
  env::Args,
  ffi::c_char,
  ops::DerefMut,
  path::{Path, PathBuf},
  sync::Arc,
};

use ext_php_rs::{
  alloc::estrdup,
  error::Error,
  ffi::php_execute_script,
  zend::{try_catch, try_catch_first, ExecutorGlobals, SapiGlobals},
};

use http_handler::types::{Request, Response};
use http_handler::Handler;

use super::{
  sapi::{ensure_sapi, Sapi},
  scopes::{FileHandleScope, RequestScope, ThreadScope},
  strings::translate_path,
  EmbedRequestError, EmbedStartError, RequestContext,
};

/// Extension type to keep the blocking PHP task alive while the response is being consumed
#[derive(Clone)]
pub struct BlockingTaskHandle(
  Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<Result<(), EmbedRequestError>>>>>,
);

impl Drop for BlockingTaskHandle {
  fn drop(&mut self) {
    // Only wait if this is the last reference
    if Arc::strong_count(&self.0) == 1 {
      // CRITICAL: We must wait for the blocking task to complete, not abort it.
      // The blocking task contains ThreadScope which must call ext_php_rs_sapi_per_thread_shutdown()
      // to properly clean up PHP's thread-local storage. If we abort, the cleanup never happens
      // and PHP's TSRM can be left in an inconsistent state, causing memory corruption.
      if let Ok(mut guard) = self.0.try_lock() {
        if let Some(handle) = guard.take() {
          // Use block_on to wait for the task to complete
          // This ensures ThreadScope::drop() runs and PHP TLS is cleaned up
          let _ = crate::sapi::fallback_handle().block_on(handle);
        }
      }
    }
  }
}

/// A simple trait for rewriting requests that works with our specific request type
pub trait RequestRewriter: Send + Sync {
  /// Rewrite the given request and return the modified request
  ///
  /// The docroot parameter is used by conditions like ExistenceCondition that need
  /// to check for files on the filesystem.
  fn rewrite_request(
    &self,
    request: Request,
    docroot: &Path,
  ) -> Result<Request, http_rewriter::RewriteError>;
}

/// Blanket implementation: any type implementing http_rewriter::Rewriter
/// automatically implements RequestRewriter for our concrete Request type.
impl<T> RequestRewriter for T
where
  T: http_rewriter::Rewriter,
{
  fn rewrite_request(
    &self,
    request: Request,
    docroot: &Path,
  ) -> Result<Request, http_rewriter::RewriteError> {
    use http_handler::extensions::DocumentRoot;
    use http_handler::RequestExt;
    let mut request = request;
    request.set_document_root(DocumentRoot {
      path: docroot.to_path_buf(),
    });
    http_rewriter::Rewriter::rewrite(self, request)
  }
}

/// Embed a PHP script into a Rust application to handle HTTP requests.
pub struct Embed {
  docroot: PathBuf,
  args: Vec<String>,

  // NOTE: This needs to hold the SAPI to keep it alive
  #[allow(dead_code)]
  sapi: Arc<Sapi>,

  rewriter: Option<Box<dyn RequestRewriter>>,
}

impl std::fmt::Debug for Embed {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Embed")
      .field("docroot", &self.docroot)
      .field("args", &self.args)
      .field("sapi", &self.sapi)
      .field("rewriter", &"Box<dyn RequestRewriter>")
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
  pub fn new<C>(
    docroot: C,
    rewriter: Option<Box<dyn RequestRewriter>>,
  ) -> Result<Self, EmbedStartError>
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
    rewriter: Option<Box<dyn RequestRewriter>>,
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
    rewriter: Option<Box<dyn RequestRewriter>>,
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

  /// Handles an HTTP request with streaming response.
  ///
  /// Returns immediately after headers are sent, with body chunks streaming asynchronously.
  /// Buffering is handled externally by NAPI Task compute() methods when needed.
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
  /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
  /// let body = http_handler::RequestBody::new();
  ///
  /// // Close the request body stream - callers must always shutdown the stream before calling handle()
  /// {
  ///   use tokio::io::AsyncWriteExt;
  ///   let mut body_writer = body.clone();
  ///   body_writer.shutdown().await.expect("should close request body stream");
  /// }
  ///
  /// let request = http_handler::request::Request::builder()
  ///   .method("GET")
  ///   .uri("http://example.com/index.php")
  ///   .body(body)
  ///   .expect("should build request");
  ///
  /// let response = handler.handle(request)
  ///   .await
  ///   .expect("should handle request");
  ///
  /// // Consume the streaming response body to ensure PHP task completes
  /// use http_body_util::BodyExt;
  /// let (_parts, body) = response.into_parts();
  /// let mut stream = body;
  /// while let Some(frame_result) = stream.frame().await {
  ///   match frame_result {
  ///     Ok(_) => continue,
  ///     Err(e) => panic!("Error reading response: {}", e),
  ///   }
  /// }
  ///
  /// drop(handler);
  /// # });
  /// ```
  /// Handle a PHP request with streaming response.
  ///
  /// Returns immediately after headers are sent, with body chunks streaming asynchronously.
  /// All buffering is external to this method, handled by NAPI Task compute() methods.
  async fn handle(&self, request: Request) -> Result<Response, Self::Error> {
    use tokio::sync::oneshot;

    // Get REQUEST_URI _first_ as it needs the pre-rewrite state.
    let uri = request.uri().clone();
    let request_uri_str = uri.path().to_string();

    // Apply request rewriting rules
    let request = if let Some(rewriter) = &self.rewriter {
      rewriter
        .rewrite_request(request, &self.docroot)
        .map_err(|e| EmbedRequestError::RequestRewriteError(e.to_string()))?
    } else {
      request
    };

    // Clone headers as owned HashMap
    let headers_map: std::collections::HashMap<String, String> = request
      .headers()
      .iter()
      .filter_map(|(k, v)| {
        v.to_str()
          .ok()
          .map(|val| (k.as_str().to_string(), val.to_string()))
      })
      .collect();

    // Translate path on async thread
    let docroot = self.docroot.clone();
    let translated_path_str = translate_path(&docroot, request.uri().path())?
      .display()
      .to_string();

    // Extract request method, query string, and headers
    let method_str = request.method().as_str().to_string();
    let query_str = uri.query().unwrap_or("").to_string();

    // Extract content-type and content-length as owned strings before spawn_blocking
    let content_type_str = headers_map
      .get("content-type")
      .or_else(|| headers_map.get("Content-Type"))
      .cloned();

    let content_length = headers_map
      .get("content-length")
      .or_else(|| headers_map.get("Content-Length"))
      .and_then(|s| s.parse::<i64>().ok())
      .unwrap_or(-1); // -1 means unknown length for streaming requests

    // Clone args as owned Strings to send to blocking thread
    let args: Vec<String> = self.args.iter().map(|s| s.to_string()).collect();

    // Create streaming response body
    let response_body = request.body().create_response();
    let response_writer = response_body.clone();

    // Channel to receive headers + status + custom headers + logs when ready (send owned data)
    let (headers_sent_tx, headers_sent_rx) =
      oneshot::channel::<(u16, String, Vec<(String, String)>, bytes::Bytes)>();

    // CRITICAL: Clone Arc<Sapi> to keep it alive while the blocking task runs.
    // If Embed is dropped before the blocking task completes, we need to prevent
    // Sapi::drop() from calling tsrm_shutdown() while PHP operations are in progress.
    let sapi = self.sapi.clone();

    // Spawn blocking PHP execution - ALL PHP operations happen here
    let blocking_handle = tokio::task::spawn_blocking(move || {
      // Keep sapi alive for the duration of the blocking task
      let _sapi = sapi;

      // Initialize thread-local storage for this worker thread.
      // This calls ext_php_rs_sapi_per_thread_init() -> ts_resource(0) which sets up
      // PHP's thread-local storage for the current thread.
      //
      // NOTE: php_module_startup() is called ONCE on the main thread when Sapi is created.
      // Worker threads only need per-thread TLS initialization via ThreadScope, NOT
      // another php_module_startup call. Calling php_module_startup from multiple threads
      // concurrently corrupts global state (memory allocator function pointers).
      let _thread_scope = ThreadScope::new();

      // Setup RequestContext (always streaming from SAPI perspective)
      // RequestContext::new() will extract the request body's read stream and add it as RequestStream extension
      let ctx = RequestContext::new(
        request,
        docroot.clone(),
        response_writer.clone(),
        headers_sent_tx,
      );
      RequestContext::set_current(Box::new(ctx));

      // All estrdup calls happen here, inside spawn_blocking, after ThreadScope::new()
      // has initialized PHP's thread-local storage. These will be freed by efree in
      // sapi_module_deactivate during request shutdown.
      let request_uri_c = estrdup(request_uri_str);
      let path_translated = estrdup(translated_path_str.clone());
      let request_method = estrdup(method_str);
      let query_string = estrdup(query_str);
      let content_type = content_type_str
        .map(estrdup)
        .unwrap_or(std::ptr::null_mut());

      // Prepare argv pointers
      let argc = args.len() as i32;
      let mut argv_ptrs: Vec<*mut c_char> = args.iter().map(|s| estrdup(s.as_str())).collect();

      // Set SAPI globals BEFORE php_request_startup since PHP reads these during initialization
      {
        let mut globals = SapiGlobals::get_mut();

        // Reset state
        globals.options |= ext_php_rs::ffi::SAPI_OPTION_NO_CHDIR as i32;
        globals.request_info.proto_num = 110;
        globals.request_info.argc = argc;
        globals.request_info.argv = argv_ptrs.as_mut_ptr();
        globals.request_info.headers_read = false;
        globals.sapi_headers.http_response_code = 200;

        // Set request info from request
        globals.request_info.request_method = request_method;
        globals.request_info.query_string = query_string;
        globals.request_info.path_translated = path_translated;
        globals.request_info.request_uri = request_uri_c;

        // TODO: Add auth fields

        globals.request_info.content_type = content_type;
        globals.request_info.content_length = content_length;
      }

      let result = try_catch_first(|| {
        let _request_scope = RequestScope::new()?;

        // Execute PHP script
        {
          let mut file_handle = FileHandleScope::new(translated_path_str.clone());
          try_catch(|| unsafe { php_execute_script(file_handle.deref_mut()) })
            .map_err(|_| EmbedRequestError::Bailout)?;
        }

        // Handle exceptions
        if let Some(err) = ExecutorGlobals::take_exception() {
          let ex = Error::Exception(err);
          return Err(EmbedRequestError::Exception(ex.to_string()));
        }

        Ok(())
        // RequestScope drops here, triggering request shutdown
        // Output buffering flush happens during shutdown, calling ub_write
        // RequestContext must still be alive at this point!
      });

      // Reclaim RequestContext AFTER RequestScope has dropped
      // This ensures output buffer flush during shutdown can still access the context
      // Note: reclaim() also shuts down the response stream to signal EOF to consumers
      let _ctx = RequestContext::reclaim();

      // Flatten the result
      match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(EmbedRequestError::Bailout),
      }
    });

    // Wait for headers to be sent (with owned status, mimetype, custom headers, and logs)
    // The JavaScript code should call req.end() concurrently using Promise.all to avoid deadlock
    let (status, mime_str, custom_headers, logs) = headers_sent_rx
      .await
      .map_err(|_| EmbedRequestError::ResponseBuildError)?;

    // Build response with headers and streaming body (on async thread, using owned data)
    let mut builder = http_handler::response::Builder::new()
      .status(status)
      .header("Content-Type", mime_str);

    // Add custom headers from PHP header() calls
    for (name, value) in custom_headers {
      builder = builder.header(name, value);
    }

    let mut response = builder
      .body(response_body)
      .map_err(|_| EmbedRequestError::ResponseBuildError)?;

    // Store logs in extensions for streaming mode (available but not streamed)
    if !logs.is_empty() {
      response
        .extensions_mut()
        .insert(http_handler::ResponseLog::from_bytes(logs));
    }

    // Store the blocking task handle to keep it alive while response is consumed
    response
      .extensions_mut()
      .insert(BlockingTaskHandle(Arc::new(tokio::sync::Mutex::new(Some(
        blocking_handle,
      )))));

    Ok(response)
  }
}

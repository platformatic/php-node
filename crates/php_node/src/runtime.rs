use std::{ops::Deref, sync::Arc};

use napi::bindgen_prelude::*;
use napi::{Env, Error, Result, Task};

use php::{Embed, EmbedRequestError, Handler, Request, Response, RequestRewriter};
use http_handler::napi::{Request as PhpRequest, Response as PhpResponse};
use http_rewriter::napi::Rewriter as NapiRewriter;

/// Options for creating a new PHP instance.
#[napi(object)]
#[derive(Default)]
pub struct PhpOptions {
  /// The command-line arguments for the PHP instance.
  pub argv: Option<Vec<String>>,
  /// The document root for the PHP instance.
  pub docroot: Option<String>,
  /// Throw request errors
  pub throw_request_errors: Option<bool>,
  /// Request rewriter
  pub rewriter: Option<Reference<NapiRewriter>>,
}

/// A PHP instance.
///
/// # Examples
///
/// ```js
/// const php = new Php({
///  code: 'echo "Hello, world!";'
/// });
///
/// const response = php.handleRequest(new Request({
///   method: 'GET',
///   url: 'http://example.com'
/// }));
///
/// console.log(response.status);
/// console.log(response.body);
/// ```
#[napi(js_name = "Php")]
pub struct PhpRuntime {
  embed: Arc<Embed>,
  throw_request_errors: bool,
}

#[napi]
impl PhpRuntime {
  /// Create a new PHP instance.
  ///
  /// # Examples
  ///
  /// ```js
  /// const php = new Php({
  ///   docroot: process.cwd(),
  ///   argv: process.argv
  /// });
  /// ```
  #[napi(constructor)]
  pub fn new(options: Option<PhpOptions>) -> Result<Self> {
    let PhpOptions {
      docroot,
      argv,
      throw_request_errors,
      rewriter,
    } = options.unwrap_or_default();

    let docroot = docroot
      .ok_or_else(|| {
        std::env::current_dir()
          .map(|s| s.display().to_string())
          .ok()
      })
      .map_err(|_| Error::from_reason("Could not determine docroot"))?;

    let rewriter = if let Some(rewriter_ref) = rewriter {
      // Dereference to get the actual NapiRewriter and clone it
      let owned_rewriter = (*rewriter_ref).clone();
      Some(Box::new(NapiRewriterWrapper(owned_rewriter)) as Box<dyn RequestRewriter>)
    } else {
      None
    };

    let embed = match argv {
      Some(argv) => Embed::new_with_argv(docroot, rewriter, argv),
      None => Embed::new(docroot, rewriter),
    }
    .map_err(|err| Error::from_reason(err.to_string()))?;

    Ok(Self {
      embed: Arc::new(embed),
      throw_request_errors: throw_request_errors.unwrap_or_default(),
    })
  }

  /// Handle a PHP request.
  ///
  /// # Examples
  ///
  /// ```js
  /// const php = new Php({
  ///   docroot: process.cwd(),
  ///   argv: process.argv
  /// });
  ///
  /// const response = php.handleRequest(new Request({
  ///   method: 'GET',
  ///   url: 'http://example.com'
  /// }));
  ///
  /// console.log(response.status);
  /// console.log(response.body);
  /// ```
  #[napi]
  pub fn handle_request(
    &self,
    request: &PhpRequest,
    signal: Option<AbortSignal>,
  ) -> AsyncTask<PhpRequestTask> {
    AsyncTask::with_optional_signal(
      PhpRequestTask {
        throw_request_errors: self.throw_request_errors,
        embed: self.embed.clone(),
        request: request.deref().clone(),
      },
      signal,
    )
  }

  /// Handle a PHP request synchronously.
  ///
  /// # Examples
  ///
  /// ```js
  /// const php = new Php({
  ///   docroot: process.cwd(),
  ///   argv: process.argv
  /// });
  ///
  /// const response = php.handleRequestSync(new Request({
  ///   method: 'GET',
  ///   url: 'http://example.com'
  /// }));
  ///
  /// console.log(response.status);
  /// console.log(response.body);
  /// ```
  #[napi]
  pub fn handle_request_sync(&self, request: &PhpRequest) -> Result<PhpResponse> {
    let mut task = PhpRequestTask {
      throw_request_errors: self.throw_request_errors,
      embed: self.embed.clone(),
      request: request.deref().clone(),
    };

    task.compute().map(Into::<PhpResponse>::into)
  }
}

// Task container to run a PHP request in a worker thread.
pub struct PhpRequestTask {
  embed: Arc<Embed>,
  request: Request,
  throw_request_errors: bool,
}

#[napi]
impl Task for PhpRequestTask {
  type Output = Response;
  type JsValue = PhpResponse;

  // Handle the PHP request in the worker thread.
  fn compute(&mut self) -> Result<Self::Output> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| Error::from_reason(e.to_string()))?;
    let mut result = runtime.block_on(self.embed.handle(self.request.clone()));

    // Translate the various error types into HTTP error responses
    if !self.throw_request_errors {
      result = result.or_else(|err| {
        Ok(match err {
          EmbedRequestError::ScriptNotFound(_script_name) => {
            http_handler::response::Builder::new()
              .status(404)
              .body(bytes::BytesMut::from("Not Found"))
              .unwrap()
          }
          _ => {
            http_handler::response::Builder::new()
              .status(500)
              .body(bytes::BytesMut::from("Internal Server Error"))
              .unwrap()
          }
        })
      })
    }

    result.map_err(|err| Error::from_reason(err.to_string()))
  }

  // Handle converting the PHP response to a JavaScript response in the main thread.
  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(Into::<PhpResponse>::into(output))
  }
}

// Wrapper to adapt NapiRewriter to RequestRewriter
struct NapiRewriterWrapper(NapiRewriter);

impl RequestRewriter for NapiRewriterWrapper {
  fn rewrite_request(&self, request: Request) -> std::result::Result<Request, http_rewriter::RewriteError> {
    // Call the Rewriter trait method explicitly  
    <NapiRewriter as http_rewriter::Rewriter>::rewrite(&self.0, request)
  }
}

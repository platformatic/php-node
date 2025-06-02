use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi::{Env, Error, Result, Task};

use php::{Embed, EmbedRequestError, Handler, Request, Response};

use crate::{PhpRequest, PhpResponse};

/// Options for creating a new PHP instance.
#[napi(object)]
#[derive(Clone, Default)]
pub struct PhpOptions {
  /// The command-line arguments for the PHP instance.
  pub argv: Option<Vec<String>>,
  /// The document root for the PHP instance.
  pub docroot: Option<String>,
  /// Throw request errors
  pub throw_request_errors: Option<bool>,
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
    } = options.unwrap_or_default();

    let docroot = docroot
      .ok_or_else(|| {
        std::env::current_dir()
          .map(|s| s.display().to_string())
          .ok()
      })
      .map_err(|_| Error::from_reason("Could not determine docroot"))?;

    let embed = match argv {
      Some(argv) => Embed::new_with_argv(docroot, argv),
      None => Embed::new(docroot),
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
        request: request.into(),
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
      request: request.into(),
    };

    task.compute().map(PhpResponse::new)
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
    let mut result = self.embed.handle(self.request.clone());

    // Translate the various error types into HTTP error responses
    if !self.throw_request_errors {
      result = result.or_else(|err| {
        Ok(match err {
          EmbedRequestError::ScriptNotFound(_script_name) => {
            Response::builder().status(404).body("Not Found").build()
          }
          _ => Response::builder()
            .status(500)
            .body("Internal Server Error")
            .build(),
        })
      })
    }

    result.map_err(|err| Error::from_reason(err.to_string()))
  }

  // Handle converting the PHP response to a JavaScript response in the main thread.
  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(PhpResponse::new(output))
  }
}

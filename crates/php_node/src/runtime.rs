use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi::{Env, Error, Result, Task};

use php::{Embed, Handler, Request, Response};

use crate::{PhpRequest, PhpResponse};

/// Options for creating a new PHP instance.
#[napi(object)]
#[derive(Clone, Default)]
pub struct PhpOptions {
  /// The command-line arguments for the PHP instance.
  pub argv: Option<Vec<String>>,
  /// The PHP code to embed.
  pub code: String,
  /// The filename for the PHP code.
  pub file: Option<String>,
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
}

#[napi]
impl PhpRuntime {
  /// Create a new PHP instance.
  ///
  /// # Examples
  ///
  /// ```js
  /// const php = new Php({
  ///   code: 'echo "Hello, world!";'
  /// });
  /// ```
  #[napi(constructor)]
  pub fn new(options: PhpOptions) -> Self {
    let code = options.code.clone();
    let filename = options.file.clone();
    let argv = options.argv.clone();

    // TODO: Need to figure out how to send an Embed across threads
    // so we can reuse the same Embed instance for multiple requests.
    let embed = match argv {
      Some(argv) => Embed::new_with_argv(code, filename, argv),
      None => Embed::new(code, filename),
    };

    Self {
      embed: Arc::new(embed),
    }
  }

  /// Handle a PHP request.
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
  #[napi]
  pub fn handle_request(&self, request: &PhpRequest) -> AsyncTask<PhpRequestTask> {
    AsyncTask::new(PhpRequestTask {
      embed: self.embed.clone(),
      request: request.into(),
    })
  }

  /// Handle a PHP request synchronously.
  ///
  /// # Examples
  ///
  /// ```js
  /// const php = new Php({
  ///   code: 'echo "Hello, world!";'
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
}

impl Task for PhpRequestTask {
  type Output = Response;
  type JsValue = PhpResponse;

  // Handle the PHP request in the worker thread.
  fn compute(&mut self) -> Result<Self::Output> {
    self
      .embed
      .handle(self.request.clone())
      .map_err(|err| Error::from_reason(err.to_string()))
  }

  // Handle converting the PHP response to a JavaScript response in the main thread.
  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(PhpResponse::new(output))
  }
}

use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi::{Env, Error, Result, Task};

use crate::sapi::fallback_handle;
use crate::{Embed, EmbedRequestError, Handler, RequestRewriter};
use crate::{Request, Response};
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
    // Register our fallback runtime with napi-rs so async tasks use the same runtime.
    // This must be called before napi-rs creates its own runtime.
    napi::bindgen_prelude::create_custom_tokio_runtime(
      tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"),
    );

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
      // Thanks to the blanket impl, NapiRewriter automatically implements RequestRewriter
      Some(Box::new(owned_rewriter) as Box<dyn RequestRewriter>)
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
    request: PhpRequest,
    signal: Option<AbortSignal>,
  ) -> AsyncTask<PhpRequestTask> {
    AsyncTask::with_optional_signal(
      PhpRequestTask {
        throw_request_errors: self.throw_request_errors,
        embed: self.embed.clone(),
        request: Some(request.into_inner()),
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
  pub fn handle_request_sync(&self, request: PhpRequest) -> Result<PhpResponse> {
    let mut task = PhpRequestTask {
      throw_request_errors: self.throw_request_errors,
      embed: self.embed.clone(),
      request: Some(request.into_inner()),
    };

    task.compute().map(Into::<PhpResponse>::into)
  }

  /// Handle a streaming PHP request.
  ///
  /// Returns immediately after headers are sent, with body chunks streaming
  /// asynchronously. Use the AsyncIterator interface to read the response body.
  ///
  /// # Examples
  ///
  /// ```js
  /// const php = new Php({
  ///   docroot: process.cwd(),
  ///   argv: process.argv
  /// });
  ///
  /// const response = await php.handleStream(new Request({
  ///   method: 'GET',
  ///   url: 'http://example.com'
  /// }));
  ///
  /// console.log(response.status);   // Available immediately
  /// console.log(response.headers);  // Available immediately
  ///
  /// // Read streaming body
  /// for await (const chunk of response) {
  ///   console.log('Chunk:', chunk.toString());
  /// }
  /// ```
  #[napi]
  pub fn handle_stream(
    &self,
    request: PhpRequest,
    signal: Option<AbortSignal>,
  ) -> AsyncTask<PhpStreamTask> {
    AsyncTask::with_optional_signal(
      PhpStreamTask {
        throw_request_errors: self.throw_request_errors,
        embed: self.embed.clone(),
        request: Some(request.into_inner()),
      },
      signal,
    )
  }
}

/// Task container to run a PHP request in a worker thread.
pub struct PhpRequestTask {
  embed: Arc<Embed>,
  request: Option<Request>,
  throw_request_errors: bool,
}

#[napi]
impl Task for PhpRequestTask {
  type Output = Response;
  type JsValue = PhpResponse;

  // Handle the PHP request in the worker thread.
  fn compute(&mut self) -> Result<Self::Output> {
    let request = self
      .request
      .take()
      .ok_or_else(|| Error::from_reason("Request already consumed"))?;

    // Extract body buffer and body handle before moving request
    let body_buffer_data: Option<bytes::Bytes> =
      if let Some(buf) = request.extensions().get::<http_handler::BodyBuffer>() {
        if !buf.is_empty() {
          Some(bytes::Bytes::copy_from_slice(buf.as_bytes()))
        } else {
          None
        }
      } else {
        None
      };
    let request_body = request.body().clone();

    // Write request body concurrently with handle() to avoid thread pool exhaustion
    // If a BodyBuffer is present, write and close the stream automatically
    // Otherwise, JavaScript code must call request.end() to close the stream
    let _write_handle = fallback_handle().spawn(async move {
      use tokio::io::AsyncWriteExt;

      let mut body = request_body;

      // If there's a BodyBuffer extension, write it (if not empty)
      if let Some(data) = body_buffer_data {
        let _ = body.write_all(&data).await;
      }

      // Always close the stream, even when there's no body
      let _ = body.shutdown().await;
    });

    // Call handle() which returns a streaming response
    // We need to buffer it here for handleRequest/handleRequestSync backward compatibility
    // Use fallback_handle() to avoid deadlocks in spawn_blocking contexts
    let mut result = fallback_handle().block_on(async {
      use http_body_util::BodyExt;

      let response = self.embed.handle(request).await?;

      // Buffer the streaming body for backward compatibility
      let (parts, body) = response.into_parts();

      // Collect body chunks, capturing any exceptions sent through the stream
      let mut body_buffer = bytes::BytesMut::new();
      let mut exception: Option<String> = None;

      let mut stream = body;
      loop {
        match stream.frame().await {
          Some(frame_result) => match frame_result {
            Ok(frame) => {
              if let Ok(data) = frame.into_data() {
                body_buffer.extend_from_slice(&data);
              }
            }
            Err(e) => {
              exception = Some(e.to_string());
              break;
            }
          },
          None => break,
        }
      }

      let mut response = Response::from_parts(parts, http_handler::ResponseBody::new());

      // Add buffered body to extensions
      response
        .extensions_mut()
        .insert(http_handler::BodyBuffer::from_bytes(body_buffer.freeze()));

      // Add exception to extensions if one occurred
      if let Some(ex) = exception {
        response
          .extensions_mut()
          .insert(http_handler::ResponseException(ex));
      }

      Ok::<_, EmbedRequestError>(response)
    });

    // Translate the various error types into HTTP error responses
    if !self.throw_request_errors {
      result = result.or_else(|err| {
        let (mut response, body_content) = match err {
          EmbedRequestError::ScriptNotFound(_script_name) => (
            http_handler::response::Builder::new()
              .status(404)
              .body(http_handler::ResponseBody::new())
              .unwrap(),
            "Not Found",
          ),
          _ => (
            http_handler::response::Builder::new()
              .status(500)
              .body(http_handler::ResponseBody::new())
              .unwrap(),
            "Internal Server Error",
          ),
        };

        // Add body content as BodyBuffer extension
        response
          .extensions_mut()
          .insert(http_handler::BodyBuffer::from_bytes(body_content));

        Ok(response)
      })
    }

    result.map_err(|err| Error::from_reason(err.to_string()))
  }

  // Handle converting the PHP response to a JavaScript response in the main thread.
  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(Into::<PhpResponse>::into(output))
  }
}

/// Task container to run a PHP streaming request in a worker thread.
pub struct PhpStreamTask {
  embed: Arc<Embed>,
  request: Option<Request>,
  throw_request_errors: bool,
}

#[napi]
impl Task for PhpStreamTask {
  type Output = Response;
  type JsValue = Object<'static>;

  // Handle the PHP streaming request in the worker thread.
  fn compute(&mut self) -> Result<Self::Output> {
    // Take ownership of the request to preserve the body channels
    let request = self
      .request
      .take()
      .ok_or_else(|| Error::from_reason("Request already consumed"))?;

    // Extract body buffer and body handle before moving request
    let body_buffer_data: Option<bytes::Bytes> =
      if let Some(buf) = request.extensions().get::<http_handler::BodyBuffer>() {
        if !buf.is_empty() {
          Some(bytes::Bytes::copy_from_slice(buf.as_bytes()))
        } else {
          None
        }
      } else {
        None
      };
    let has_body_buffer = body_buffer_data.is_some();
    let request_body = request.body().clone();

    // Write request body concurrently with handle() to avoid thread pool exhaustion
    // If a BodyBuffer is present, write and close the stream automatically
    // Otherwise, JavaScript code must call request.end() to close the stream
    let _write_handle = fallback_handle().spawn(async move {
      use tokio::io::AsyncWriteExt;

      let mut body = request_body;

      // If body was provided in constructor (BodyBuffer exists), write it and close the stream
      if let Some(data) = body_buffer_data {
        let _ = body.write_all(&data).await;
      }
      // Close the stream if body was fully provided
      if has_body_buffer {
        let _ = body.shutdown().await;
      }
      // If no BodyBuffer, JavaScript will write via req.write() and close via req.end()
      // Don't touch the stream here to avoid "broken pipe" errors
    });

    // Use fallback_handle() to avoid deadlocks in spawn_blocking contexts
    let mut result = fallback_handle().block_on(async {
      // Let write task run concurrently with handle()
      self.embed.handle(request).await
    });

    // Translate the various error types into HTTP error responses
    if !self.throw_request_errors {
      result = result.or_else(|err| {
        let (mut response, body_content) = match err {
          EmbedRequestError::ScriptNotFound(_script_name) => (
            http_handler::response::Builder::new()
              .status(404)
              .body(http_handler::ResponseBody::new())
              .unwrap(),
            "Not Found",
          ),
          _ => (
            http_handler::response::Builder::new()
              .status(500)
              .body(http_handler::ResponseBody::new())
              .unwrap(),
            "Internal Server Error",
          ),
        };

        // Store body content in BodyBuffer extension
        response
          .extensions_mut()
          .insert(http_handler::BodyBuffer::from_bytes(bytes::Bytes::from(
            body_content,
          )));

        Ok(response)
      });
    }

    result.map_err(|e| Error::from_reason(e.to_string()))
  }

  // Handle converting the PHP response to a JavaScript response in the main thread.
  fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
    let response: PhpResponse = Into::<PhpResponse>::into(output);
    response.make_streamable(env)
  }
}

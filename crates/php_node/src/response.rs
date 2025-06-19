use napi::bindgen_prelude::*;
use napi::Result;

use php::Response;

use crate::PhpHeaders;

/// Options for creating a new PHP response.
#[napi(object)]
#[derive(Default)]
pub struct PhpResponseOptions {
  /// The HTTP status code for the response.
  pub status: Option<i32>,
  /// The headers for the response.
  pub headers: Option<PhpHeaders>,
  /// The body for the response.
  pub body: Option<Uint8Array>,
  /// The log for the response.
  pub log: Option<Uint8Array>,
  /// The exception for the response.
  pub exception: Option<String>,
}

/// A PHP response.
#[napi(js_name = "Response")]
pub struct PhpResponse {
  response: Response,
}

impl PhpResponse {
  // Create a new PHP response instance.
  pub fn new(response: Response) -> Self {
    PhpResponse { response }
  }
}

#[napi]
impl PhpResponse {
  /// Create a new PHP response.
  ///
  /// # Examples
  ///
  /// ```js
  /// const response = new Response({
  ///   status: 200,
  ///   headers: {
  ///     'Content-Type': ['application/json']
  ///   },
  ///   body: new Uint8Array([1, 2, 3, 4])
  /// });
  /// ```
  #[napi(constructor)]
  pub fn constructor(options: Option<PhpResponseOptions>) -> Result<Self> {
    let options = options.unwrap_or_default();
    let mut builder = Response::builder();

    if let Some(status) = options.status {
      builder.status(status);
    }

    if let Some(headers) = options.headers {
      builder = builder.headers(headers);
    }

    if let Some(body) = options.body {
      builder.body(body.as_ref());
    }

    if let Some(log) = options.log {
      builder.log(log.as_ref());
    }

    if let Some(exception) = options.exception {
      builder.exception(exception);
    }

    Ok(PhpResponse {
      response: builder.build(),
    })
  }

  /// Get the HTTP status code for the response.
  ///
  /// # Examples
  ///
  /// ```js
  /// const response = new Response({
  ///   status: 200
  /// });
  ///
  /// console.log(response.status);
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn status(&self) -> u32 {
    self.response.status() as u32
  }

  /// Get the headers for the response.
  ///
  /// # Examples
  ///
  /// ```js
  /// const response = new Response({
  ///   headers: {
  ///     'Content-Type': ['application/json']
  ///   }
  /// });
  ///
  /// for (const mime of response.headers.get('Content-Type')) {
  ///   console.log(mime);
  /// }
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn headers(&self) -> PhpHeaders {
    PhpHeaders::new(self.response.headers().clone())
  }

  /// Get the body for the response.
  ///
  /// # Examples
  ///
  /// ```js
  /// const response = new Response({
  ///   body: new Uint8Array([1, 2, 3, 4])
  /// });
  ///
  /// console.log(response.body);
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn body(&self) -> Buffer {
    self.response.body().to_vec().into()
  }

  /// Get the log for the response.
  ///
  /// # Examples
  ///
  /// ```js
  /// const response = new Response({
  ///   log: new Uint8Array([1, 2, 3, 4])
  /// });
  ///
  /// console.log(response.log);
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn log(&self) -> Buffer {
    self.response.log().to_vec().into()
  }

  /// Get the exception for the response.
  ///
  /// # Examples
  ///
  /// ```js
  /// const response = new Response({
  ///   exception: 'An error occurred'
  /// });
  ///
  /// console.log(response.exception);
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn exception(&self) -> Option<String> {
    self.response.exception().map(|v| v.to_owned())
  }
}

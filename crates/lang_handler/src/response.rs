use bytes::{Bytes, BytesMut};

use super::Headers;

/// Represents an HTTP response. This includes the status code, headers, body, log, and exception.
///
/// # Example
///
/// ```
/// use lang_handler::{Response, ResponseBuilder};
///
/// let response = Response::builder()
///   .status(200)
///   .header("Content-Type", "text/plain")
///   .body("Hello, World!")
///   .build();
///
/// assert_eq!(response.status(), 200);
/// assert_eq!(response.headers().get("Content-Type"), Some("text/plain".to_string()));
/// assert_eq!(response.body(), "Hello, World!");
/// ```
#[derive(Clone, Debug)]
pub struct Response {
  status: i32,
  headers: Headers,
  // TODO: Support Stream bodies when napi.rs supports it
  body: Bytes,
  log: Bytes,
  exception: Option<String>,
}

impl Response {
  /// Creates a new response with the given status code, headers, body, log, and exception.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::{Response, Headers};
  ///
  /// let mut headers = Headers::new();
  /// headers.set("Content-Type", "text/plain");
  ///
  /// let response = Response::new(200, headers, "Hello, World!", "log", Some("exception".to_string()));
  ///
  /// assert_eq!(response.status(), 200);
  /// assert_eq!(response.headers().get("Content-Type"), Some("text/plain".to_string()));
  /// assert_eq!(response.body(), "Hello, World!");
  /// assert_eq!(response.log(), "log");
  /// assert_eq!(response.exception(), Some(&"exception".to_string()));
  /// ```
  pub fn new<B, L>(
    status: i32,
    headers: Headers,
    body: B,
    log: L,
    exception: Option<String>,
  ) -> Self
  where
    B: Into<Bytes>,
    L: Into<Bytes>,
  {
    Self {
      status,
      headers,
      body: body.into(),
      log: log.into(),
      exception,
    }
  }

  /// Creates a new response builder.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::Response;
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .header("Content-Type", "text/plain")
  ///   .body("Hello, World!")
  ///   .build();
  ///
  /// assert_eq!(response.status(), 200);
  /// assert_eq!(response.headers().get("Content-Type"), Some("text/plain".to_string()));
  /// assert_eq!(response.body(), "Hello, World!");
  /// ```
  pub fn builder() -> ResponseBuilder {
    ResponseBuilder::new()
  }

  /// Create a new response builder that extends the given response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::{Response, ResponseBuilder};
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .header("Content-Type", "text/plain")
  ///   .body("Hello, World!")
  ///   .build();
  ///
  /// let extended = response.extend()
  ///   .status(201)
  ///   .build();
  ///
  /// assert_eq!(extended.status(), 201);
  /// assert_eq!(extended.headers().get("Content-Type"), Some("text/plain".to_string()));
  /// assert_eq!(extended.body(), "Hello, World!");
  /// ```
  pub fn extend(&self) -> ResponseBuilder {
    ResponseBuilder::extend(self)
  }

  /// Returns the status code of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::Response;
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .build();
  ///
  /// assert_eq!(response.status(), 200);
  /// ```
  pub fn status(&self) -> i32 {
    self.status
  }

  /// Returns the headers of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::{Response, Headers};
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .header("Content-Type", "text/plain")
  ///   .build();
  ///
  /// assert_eq!(response.headers().get("Content-Type"), Some("text/plain".to_string()));
  /// ```
  pub fn headers(&self) -> &Headers {
    &self.headers
  }

  /// Returns the body of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::Response;
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .body("Hello, World!")
  ///   .build();
  ///
  /// assert_eq!(response.body(), "Hello, World!");
  /// ```
  pub fn body(&self) -> Bytes {
    self.body.clone()
  }

  /// Returns the log of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::Response;
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .log("log")
  ///   .build();
  ///
  /// assert_eq!(response.log(), "log");
  /// ```
  pub fn log(&self) -> Bytes {
    self.log.clone()
  }

  /// Returns the exception of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::Response;
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .exception("exception")
  ///   .build();
  ///
  /// assert_eq!(response.exception(), Some(&"exception".to_string()));
  /// ```
  pub fn exception(&self) -> Option<&String> {
    self.exception.as_ref()
  }
}

/// A builder for creating an HTTP response.
///
/// # Example
///
/// ```
/// use lang_handler::{Response, ResponseBuilder};
///
/// let response = Response::builder()
///   .status(200)
///   .header("Content-Type", "text/plain")
///   .body("Hello, World!")
///   .build();
///
/// assert_eq!(response.status(), 200);
/// assert_eq!(response.headers().get("Content-Type"), Some("text/plain".to_string()));
/// assert_eq!(response.body(), "Hello, World!");
/// ```
#[derive(Clone, Debug)]
pub struct ResponseBuilder {
  status: Option<i32>,
  headers: Headers,
  pub(crate) body: BytesMut,
  pub(crate) log: BytesMut,
  exception: Option<String>,
}

impl ResponseBuilder {
  /// Creates a new response builder.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::ResponseBuilder;
  ///
  /// let builder = ResponseBuilder::new();
  /// ```
  pub fn new() -> Self {
    ResponseBuilder {
      status: None,
      headers: Headers::new(),
      body: BytesMut::with_capacity(1024),
      log: BytesMut::with_capacity(1024),
      exception: None,
    }
  }

  /// Creates a new response builder that extends the given response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::{Response, ResponseBuilder};
  ///
  /// let response = Response::builder()
  ///   .status(200)
  ///   .header("Content-Type", "text/plain")
  ///   .body("Hello, World!")
  ///   .build();
  ///
  /// let extended = response.extend()
  ///   .status(201)
  ///   .build();
  ///
  /// assert_eq!(extended.status(), 201);
  /// assert_eq!(extended.headers().get("Content-Type"), Some("text/plain".to_string()));
  /// assert_eq!(extended.body(), "Hello, World!");
  /// ```
  pub fn extend(response: &Response) -> Self {
    ResponseBuilder {
      status: Some(response.status),
      headers: response.headers.clone(),
      body: BytesMut::from(response.body()),
      log: BytesMut::from(response.log()),
      exception: response.exception.clone(),
    }
  }

  /// Sets the status code of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::ResponseBuilder;
  ///
  /// let response = ResponseBuilder::new()
  ///   .status(300)
  ///   .build();
  ///
  /// assert_eq!(response.status(), 300);
  /// ```
  pub fn status(&mut self, status: i32) -> &mut Self {
    self.status = Some(status);
    self
  }

  /// Sets the headers of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::ResponseBuilder;
  ///
  /// let response = ResponseBuilder::new()
  ///   .header("Content-Type", "text/plain")
  ///   .build();
  ///
  /// assert_eq!(response.headers().get("Content-Type"), Some("text/plain".to_string()));
  /// ```
  pub fn header<K, V>(&mut self, key: K, value: V) -> &mut Self
  where
    K: Into<String>,
    V: Into<String>,
  {
    self.headers.add(key, value);
    self
  }

  /// Sets the body of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::ResponseBuilder;
  ///
  /// let builder = ResponseBuilder::new()
  ///   .body("Hello, World!")
  ///   .build();
  ///
  /// assert_eq!(builder.body(), "Hello, World!");
  /// ```
  pub fn body<B: Into<BytesMut>>(&mut self, body: B) -> &mut Self {
    self.body = body.into();
    self
  }

  pub fn body_write<B: Into<BytesMut>>(&mut self, body: B) -> &mut Self {
    self.body.extend_from_slice(&body.into());
    self
  }

  /// Sets the log of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::ResponseBuilder;
  ///
  /// let builder = ResponseBuilder::new()
  ///   .log("log")
  ///   .build();
  ///
  /// assert_eq!(builder.log(), "log");
  /// ```
  pub fn log<L: Into<BytesMut>>(&mut self, log: L) -> &mut Self {
    self.log = log.into();
    self
  }

  pub fn log_write<L: Into<BytesMut>>(&mut self, log: L) -> &mut Self {
    self.log.extend_from_slice(&log.into());
    self.log.extend_from_slice(b"\n");
    self
  }

  /// Sets the exception of the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::ResponseBuilder;
  ///
  /// let builder = ResponseBuilder::new()
  ///   .exception("exception")
  ///   .build();
  ///
  /// assert_eq!(builder.exception(), Some(&"exception".to_string()));
  /// ```
  pub fn exception<E: Into<String>>(&mut self, exception: E) -> &mut Self {
    self.exception = Some(exception.into());
    self
  }

  /// Builds the response.
  ///
  /// # Example
  ///
  /// ```
  /// use lang_handler::ResponseBuilder;
  ///
  /// let response = ResponseBuilder::new()
  ///   .build();
  ///
  /// assert_eq!(response.status(), 200);
  /// assert_eq!(response.body(), "");
  /// assert_eq!(response.log(), "");
  /// assert_eq!(response.exception(), None);
  /// ```
  pub fn build(&self) -> Response {
    Response {
      status: self.status.unwrap_or(200),
      headers: self.headers.clone(),
      body: self.body.clone().freeze(),
      log: self.log.clone().freeze(),
      exception: self.exception.clone(),
    }
  }
}

impl Default for ResponseBuilder {
  fn default() -> Self {
    Self::new()
  }
}

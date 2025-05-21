use std::{
  fmt::Debug,
  net::{AddrParseError, SocketAddr},
};

use bytes::{Bytes, BytesMut};
use url::{ParseError, Url};

use crate::Headers;

/// Represents an HTTP request. Includes the method, URL, headers, and body.
///
/// # Examples
///
/// ```
/// use lang_handler::{Request, Headers};
///
/// let request = Request::builder()
///   .method("POST")
///   .url("http://example.com/test.php").expect("invalid url")
///   .header("Accept", "text/html")
///   .header("Accept", "application/json")
///   .header("Host", "example.com")
///   .body("Hello, World!")
///   .build();
///
/// assert_eq!(request.method(), "POST");
/// assert_eq!(request.url().as_str(), "http://example.com/test.php");
/// assert_eq!(request.headers().get("Accept"), Some(&vec![
///   "text/html".to_string(),
///   "application/json".to_string()
/// ]));
/// assert_eq!(request.headers().get("Host"), Some(&vec!["example.com".to_string()]));
/// assert_eq!(request.body(), "Hello, World!");
/// ```
#[derive(Clone, Debug)]
pub struct Request {
  method: String,
  url: Url,
  headers: Headers,
  // TODO: Support Stream bodies when napi.rs supports it
  body: Bytes,
  local_socket: Option<SocketAddr>,
  remote_socket: Option<SocketAddr>,
}

unsafe impl Sync for Request {}
unsafe impl Send for Request {}

impl Request {
  /// Creates a new `Request` with the given method, URL, headers, and body.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, Headers};
  ///
  /// let mut headers = Headers::new();
  /// headers.set("Accept", "text/html");
  ///
  /// let request = Request::new(
  ///   "POST".to_string(),
  ///   "http://example.com/test.php".parse().unwrap(),
  ///   headers,
  ///   "Hello, World!",
  ///   None,
  ///   None,
  /// );
  pub fn new<T: Into<Bytes>>(
    method: String,
    url: Url,
    headers: Headers,
    body: T,
    local_socket: Option<SocketAddr>,
    remote_socket: Option<SocketAddr>,
  ) -> Self {
    Self {
      method,
      url,
      headers,
      body: body.into(),
      local_socket,
      remote_socket,
    }
  }

  /// Creates a new `RequestBuilder` to build a `Request`.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, RequestBuilder};
  ///
  /// let request = Request::builder()
  ///   .method("POST")
  ///   .url("http://example.com/test.php").expect("invalid url")
  ///   .header("Content-Type", "text/html")
  ///   .header("Content-Length", 13.to_string())
  ///   .body("Hello, World!")
  ///   .build();
  ///
  /// assert_eq!(request.method(), "POST");
  /// assert_eq!(request.url().as_str(), "http://example.com/test.php");
  /// assert_eq!(request.headers().get("Content-Type"), Some(&vec!["text/html".to_string()]));
  /// assert_eq!(request.headers().get("Content-Length"), Some(&vec!["13".to_string()]));
  /// assert_eq!(request.body(), "Hello, World!");
  /// ```
  pub fn builder() -> RequestBuilder {
    RequestBuilder::new()
  }

  /// Creates a new `RequestBuilder` to extend a `Request`.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, RequestBuilder};
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com/test.php").expect("invalid url")
  ///   .header("Content-Type", "text/plain")
  ///   .build();
  ///
  /// let extended = request.extend()
  ///   .method("POST")
  ///   .header("Content-Length", 12.to_string())
  ///   .body("Hello, World")
  ///   .build();
  ///
  /// assert_eq!(extended.method(), "POST");
  /// assert_eq!(extended.url().as_str(), "http://example.com/test.php");
  /// assert_eq!(extended.headers().get("Content-Type"), Some(&vec!["text/plain".to_string()]));
  /// assert_eq!(extended.headers().get("Content-Length"), Some(&vec!["12".to_string()]));
  /// assert_eq!(extended.body(), "Hello, World");
  /// ```
  pub fn extend(&self) -> RequestBuilder {
    RequestBuilder::extend(self)
  }

  /// Returns the method of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, Headers};
  ///
  /// let request = Request::new(
  ///   "POST".to_string(),
  ///   "http://example.com/test.php".parse().unwrap(),
  ///   Headers::new(),
  ///   "Hello, World!",
  ///   None,
  ///   None,
  /// );
  ///
  /// assert_eq!(request.method(), "POST");
  /// ```
  pub fn method(&self) -> &str {
    &self.method
  }

  /// Returns the URL of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, Headers};
  ///
  /// let request = Request::new(
  ///   "POST".to_string(),
  ///   "http://example.com/test.php".parse().unwrap(),
  ///   Headers::new(),
  ///   "Hello, World!",
  ///   None,
  ///   None,
  /// );
  ///
  /// assert_eq!(request.url().as_str(), "http://example.com/test.php");
  /// ```
  pub fn url(&self) -> &Url {
    &self.url
  }

  /// Returns the headers of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, Headers};
  ///
  /// let mut headers = Headers::new();
  /// headers.set("Accept", "text/html");
  ///
  /// let request = Request::new(
  ///   "POST".to_string(),
  ///   "http://example.com/test.php".parse().unwrap(),
  ///   headers,
  ///   "Hello, World!",
  ///   None,
  ///   None,
  /// );
  ///
  /// assert_eq!(request.headers().get("Accept"), Some(&vec!["text/html".to_string()]));
  /// ```
  pub fn headers(&self) -> &Headers {
    &self.headers
  }

  /// Returns the body of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, Headers};
  ///
  /// let request = Request::new(
  ///   "POST".to_string(),
  ///   "http://example.com/test.php".parse().unwrap(),
  ///   Headers::new(),
  ///   "Hello, World!",
  ///   None,
  ///   None,
  /// );
  ///
  /// assert_eq!(request.body(), "Hello, World!");
  /// ```
  pub fn body(&self) -> Bytes {
    self.body.clone()
  }

  /// Returns the local socket address of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, Headers};
  ///
  /// let request = Request::new(
  ///   "POST".to_string(),
  ///   "http://example.com/test.php".parse().unwrap(),
  ///   Headers::new(),
  ///   "Hello, World!",
  ///   None,
  ///   None,
  /// );
  ///
  /// assert_eq!(request.local_socket(), None);
  /// ```
  pub fn local_socket(&self) -> Option<SocketAddr> {
    self.local_socket
  }

  /// Returns the remote socket address of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Request, Headers};
  ///
  /// let request = Request::new(
  ///   "POST".to_string(),
  ///   "http://example.com/test.php".parse().unwrap(),
  ///   Headers::new(),
  ///   "Hello, World!",
  ///   None,
  ///   None,
  /// );
  ///
  /// assert_eq!(request.remote_socket(), None);
  /// ```
  pub fn remote_socket(&self) -> Option<SocketAddr> {
    self.remote_socket
  }
}

/// Builds an HTTP request.
///
/// # Examples
///
/// ```
/// use lang_handler::{Request, RequestBuilder};
///
/// let request = Request::builder()
///   .method("POST")
///   .url("http://example.com/test.php").expect("invalid url")
///   .header("Content-Type", "text/html")
///   .header("Content-Length", 13.to_string())
///   .body("Hello, World!")
///   .build();
///
/// assert_eq!(request.method(), "POST");
/// assert_eq!(request.url().as_str(), "http://example.com/test.php");
/// assert_eq!(request.headers().get("Content-Type"), Some(&vec!["text/html".to_string()]));
/// assert_eq!(request.headers().get("Content-Length"), Some(&vec!["13".to_string()]));
/// assert_eq!(request.body(), "Hello, World!");
/// ```
#[derive(Clone)]
pub struct RequestBuilder {
  method: Option<String>,
  url: Option<Url>,
  headers: Headers,
  body: BytesMut,
  local_socket: Option<SocketAddr>,
  remote_socket: Option<SocketAddr>,
}

impl RequestBuilder {
  /// Creates a new `RequestBuilder`.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let builder = RequestBuilder::new();
  /// ```
  pub fn new() -> Self {
    Self {
      method: None,
      url: None,
      headers: Headers::new(),
      body: BytesMut::with_capacity(1024),
      local_socket: None,
      remote_socket: None,
    }
  }

  /// Creates a new `RequestBuilder` to extend an existing `Request`.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Headers, Request, RequestBuilder};
  ///
  /// let mut headers = Headers::new();
  /// headers.set("Accept", "text/html");
  ///
  /// let request = Request::new(
  ///   "GET".to_string(),
  ///   "http://example.com".parse().unwrap(),
  ///   headers,
  ///   "Hello, World!"
  /// );
  ///
  /// let extended = RequestBuilder::extend(&request)
  ///   .build();
  ///
  /// assert_eq!(extended.method(), "GET");
  /// assert_eq!(extended.url().as_str(), "http://example.com/");
  /// assert_eq!(extended.headers().get("Accept"), Some(&vec!["text/html".to_string()]));
  /// assert_eq!(extended.body(), "Hello, World!");
  /// ```
  pub fn extend(request: &Request) -> Self {
    Self {
      method: Some(request.method().into()),
      url: Some(request.url().clone()),
      headers: request.headers().clone(),
      body: BytesMut::from(request.body()),
      local_socket: request.local_socket.clone(),
      remote_socket: request.remote_socket.clone(),
    }
  }

  /// Sets the method of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///  .method("POST")
  ///  .build();
  ///
  /// assert_eq!(request.method(), "POST");
  /// ```
  pub fn method<T: Into<String>>(mut self, method: T) -> Self {
    self.method = Some(method.into());
    self
  }

  /// Sets the URL of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .url("http://example.com/test.php").expect("invalid url")
  ///   .build();
  ///
  /// assert_eq!(request.url().as_str(), "http://example.com/test.php");
  /// ```
  pub fn url<T>(mut self, url: T) -> Result<Self, ParseError>
  where
    T: Into<String>,
  {
    match url.into().parse() {
      Ok(url) => {
        self.url = Some(url);
        Ok(self)
      }
      Err(e) => return Err(e),
    }
  }

  /// Sets a header of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .header("Accept", "text/html")
  ///   .build();
  ///
  /// assert_eq!(request.headers().get("Accept"), Some(&vec!["text/html".to_string()]));
  /// ```
  pub fn header<K, V>(mut self, key: K, value: V) -> Self
  where
    K: Into<String>,
    V: Into<String>,
  {
    self.headers.set(key.into(), value.into());
    self
  }

  /// Sets the body of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .body("Hello, World!")
  ///   .build();
  ///
  /// assert_eq!(request.body(), "Hello, World!");
  /// ```
  pub fn body<T: Into<BytesMut>>(mut self, body: T) -> Self {
    self.body = body.into();
    self
  }

  /// Sets the local socket of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .local_socket("127.0.0.1:8080").expect("invalid local socket")
  ///   .build();
  ///
  /// assert_eq!(request.local_socket(), "127.0.0.1:8080");
  /// ```
  pub fn local_socket<T>(mut self, local_socket: T) -> Result<Self, AddrParseError>
  where
    T: Into<String>,
  {
    match local_socket.into().parse() {
      Err(e) => Err(e),
      Ok(local_socket) => {
        self.local_socket = Some(local_socket);
        Ok(self)
      }
    }
  }

  /// Sets the remote socket of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .remote_socket("127.0.0.1:8080").expect("invalid remote socket")
  ///   .build();
  ///
  /// assert_eq!(request.remote_socket(), "127.0.0.1:8080");
  /// ```
  pub fn remote_socket<T>(mut self, remote_socket: T) -> Result<Self, AddrParseError>
  where
    T: Into<String>,
  {
    match remote_socket.into().parse() {
      Err(e) => Err(e),
      Ok(remote_socket) => {
        self.remote_socket = Some(remote_socket);
        Ok(self)
      }
    }
  }

  /// Builds the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .build();
  ///
  /// assert_eq!(request.method(), "GET");
  /// assert_eq!(request.url().as_str(), "http://example.com/");
  /// assert_eq!(request.body(), "");
  /// ```
  pub fn build(self) -> Request {
    Request {
      method: self.method.unwrap_or_else(|| "GET".to_string()),
      // TODO: This is wrong. Return a Result instead.
      url: self
        .url
        .unwrap_or_else(|| Url::parse("http://example.com").unwrap()),
      headers: self.headers,
      body: self.body.freeze(),
      local_socket: self.local_socket,
      remote_socket: self.remote_socket,
    }
  }
}

use std::{fmt::Debug, net::SocketAddr};

use bytes::{Bytes, BytesMut};
use url::Url;

use super::Headers;

/// Represents an HTTP request. Includes the method, URL, headers, and body.
///
/// # Examples
///
/// ```
/// use lang_handler::{Request, Headers};
///
/// let request = Request::builder()
///   .method("POST")
///   .url("http://example.com/test.php")
///   .header("Accept", "text/html")
///   .header("Accept", "application/json")
///   .header("Host", "example.com")
///   .body("Hello, World!")
///   .build()
///   .expect("should build request");
///
/// assert_eq!(request.method(), "POST");
/// assert_eq!(request.url().as_str(), "http://example.com/test.php");
/// assert_eq!(request.headers().get_all("Accept"), vec![
///   "text/html".to_string(),
///   "application/json".to_string()
/// ]);
/// assert_eq!(request.headers().get("Host"), Some("example.com".to_string()));
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
  ///   .url("http://example.com/test.php")
  ///   .header("Content-Type", "text/html")
  ///   .header("Content-Length", 13.to_string())
  ///   .body("Hello, World!")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(request.method(), "POST");
  /// assert_eq!(request.url().as_str(), "http://example.com/test.php");
  /// assert_eq!(request.headers().get("Content-Type"), Some("text/html".to_string()));
  /// assert_eq!(request.headers().get("Content-Length"), Some("13".to_string()));
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
  ///   .url("http://example.com/test.php")
  ///   .header("Content-Type", "text/plain")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let extended = request.extend()
  ///   .method("POST")
  ///   .header("Content-Length", 12.to_string())
  ///   .body("Hello, World")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(extended.method(), "POST");
  /// assert_eq!(extended.url().as_str(), "http://example.com/test.php");
  /// assert_eq!(extended.headers().get("Content-Type"), Some("text/plain".to_string()));
  /// assert_eq!(extended.headers().get("Content-Length"), Some("12".to_string()));
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
  /// assert_eq!(request.headers().get("Accept"), Some("text/html".to_string()));
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

/// Errors which may be produced when building a Request from a RequestBuilder.
#[derive(Debug, PartialEq)]
pub enum RequestBuilderException {
  /// Url is required
  UrlMissing,
  /// Url could not be parsed
  UrlParseFailed(String),
  /// SocketAddr could not be parsed
  SocketParseFailed(String),
}

impl std::fmt::Display for RequestBuilderException {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RequestBuilderException::UrlMissing => write!(f, "Expected url to be set"),
      RequestBuilderException::UrlParseFailed(u) => write!(f, "Failed to parse url: \"{}\"", u),
      RequestBuilderException::SocketParseFailed(s) => {
        write!(f, "Failed to parse socket info: \"{}\"", s)
      }
    }
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
///   .url("http://example.com/test.php")
///   .header("Content-Type", "text/html")
///   .header("Content-Length", 13.to_string())
///   .body("Hello, World!")
///   .build()
///   .expect("should build request");
///
/// assert_eq!(request.method(), "POST");
/// assert_eq!(request.url().as_str(), "http://example.com/test.php");
/// assert_eq!(request.headers().get("Content-Type"), Some("text/html".to_string()));
/// assert_eq!(request.headers().get("Content-Length"), Some("13".to_string()));
/// assert_eq!(request.body(), "Hello, World!");
/// ```
#[derive(Clone)]
pub struct RequestBuilder {
  method: Option<String>,
  url: Option<String>,
  headers: Headers,
  body: BytesMut,
  local_socket: Option<String>,
  remote_socket: Option<String>,
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
  ///   "Hello, World!",
  ///   None,
  ///   None
  /// );
  ///
  /// let extended = RequestBuilder::extend(&request)
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(extended.method(), "GET");
  /// assert_eq!(extended.url().as_str(), "http://example.com/");
  /// assert_eq!(extended.headers().get("Accept"), Some("text/html".to_string()));
  /// assert_eq!(extended.body(), "Hello, World!");
  /// ```
  pub fn extend(request: &Request) -> Self {
    Self {
      method: Some(request.method().into()),
      url: Some(request.url().to_string()),
      headers: request.headers().clone(),
      body: BytesMut::from(request.body()),
      local_socket: request.local_socket.map(|s| s.to_string()),
      remote_socket: request.remote_socket.map(|s| s.to_string()),
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
  ///  .url("http://example.com/test.php")
  ///  .build()
  ///   .expect("should build request");
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
  ///   .url("http://example.com/test.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(request.url().as_str(), "http://example.com/test.php");
  /// ```
  pub fn url<T>(mut self, url: T) -> Self
  where
    T: Into<String>,
  {
    self.url = Some(url.into());
    self
  }

  /// Sets a header of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .url("http://example.com/test.php")
  ///   .header("Accept", "text/html")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(request.headers().get("Accept"), Some("text/html".to_string()));
  /// ```
  pub fn header<K, V>(mut self, key: K, value: V) -> Self
  where
    K: Into<String>,
    V: Into<String>,
  {
    self.headers.add(key.into(), value.into());
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
  ///   .url("http://example.com/test.php")
  ///   .body("Hello, World!")
  ///   .build()
  ///   .expect("should build request");
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
  /// use std::net::SocketAddr;
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .url("http://example.com/test.php")
  ///   .local_socket("127.0.0.1:8080")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let expected = "127.0.0.1:8080"
  ///   .parse::<SocketAddr>()
  ///   .expect("should parse");
  /// assert_eq!(request.local_socket(), Some(expected));
  /// ```
  pub fn local_socket<T>(mut self, local_socket: T) -> Self
  where
    T: Into<String>,
  {
    self.local_socket = Some(local_socket.into());
    self
  }

  /// Sets the remote socket of the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use std::net::SocketAddr;
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .url("http://example.com/test.php")
  ///   .remote_socket("127.0.0.1:8080")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let expected = "127.0.0.1:8080"
  ///   .parse::<SocketAddr>()
  ///   .expect("should parse");
  /// assert_eq!(request.remote_socket(), Some(expected));
  /// ```
  pub fn remote_socket<T>(mut self, remote_socket: T) -> Self
  where
    T: Into<String>,
  {
    self.remote_socket = Some(remote_socket.into());
    self
  }

  /// Builds the request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::RequestBuilder;
  ///
  /// let request = RequestBuilder::new()
  ///   .url("http://example.com/test.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(request.method(), "GET");
  /// assert_eq!(request.url().as_str(), "http://example.com/test.php");
  /// assert_eq!(request.body(), "");
  /// ```
  pub fn build(self) -> Result<Request, RequestBuilderException> {
    Ok(Request {
      method: self.method.unwrap_or_else(|| "GET".to_string()),
      url: parse_url(self.url)?,
      headers: self.headers,
      body: self.body.freeze(),
      local_socket: parse_socket(self.local_socket)?,
      remote_socket: parse_socket(self.remote_socket)?,
    })
  }
}

impl Default for RequestBuilder {
  fn default() -> Self {
    Self::new()
  }
}

fn parse_url(url: Option<String>) -> Result<Url, RequestBuilderException> {
  url
    .ok_or(RequestBuilderException::UrlMissing)
    .and_then(|u| {
      u.parse()
        .map_err(|_| RequestBuilderException::UrlParseFailed(u))
    })
}

fn parse_socket(socket: Option<String>) -> Result<Option<SocketAddr>, RequestBuilderException> {
  socket.map_or_else(
    || Ok(None),
    |s| {
      Ok(Some(s.parse::<SocketAddr>().map_err(|_| {
        RequestBuilderException::SocketParseFailed(s)
      })?))
    },
  )
}

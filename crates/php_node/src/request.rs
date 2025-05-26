use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi::Result;

use php::{Request, RequestBuilder};

use crate::PhpHeaders;

#[napi(object)]
#[derive(Default)]
pub struct PhpRequestSocketOptions {
  /// The string representation of the local IP address the remote client is connecting on.
  pub local_address: String,
  /// The numeric representation of the local port. For example, 80 or 21.
  pub local_port: u16,
  /// The string representation of the remote IP address.
  pub remote_address: String,
  /// The numeric representation of the remote port. For example, 80 or 21.
  pub remote_port: u16,
}

/// Options for creating a new PHP request.
#[napi(object)]
#[derive(Default)]
pub struct PhpRequestOptions {
  /// The HTTP method for the request.
  pub method: String,
  /// The URL for the request.
  pub url: String,
  /// The headers for the request.
  /// TODO: Figure out how to accept a Headers instance
  /// TODO: Figure out how to support both single values without array wrap
  pub headers: Option<HashMap<String, Vec<String>>>,
  /// The body for the request.
  pub body: Option<Uint8Array>,
  /// The socket information for the request.
  pub socket: Option<PhpRequestSocketOptions>,
}

/// A PHP request.
///
/// # Examples
///
/// ```js
/// const request = new Request({
///   method: 'GET',
///   url: 'http://example.com',
///   headers: {
///    'Content-Type': ['application/json']
///   },
///   body: new Uint8Array([1, 2, 3, 4])
/// });
/// ```
#[napi(js_name = "Request")]
pub struct PhpRequest {
  pub(crate) request: Request,
}

// Future ideas:
// - Support passing in a Node.js IncomingMessage object directly?
// - Support web standard Request objects?
#[napi]
impl PhpRequest {
  /// Create a new PHP request.
  ///
  /// # Examples
  ///
  /// ```js
  /// const request = new Request({
  ///   method: 'GET',
  ///   url: 'http://example.com',
  ///   headers: {
  ///     'Content-Type': ['application/json']
  ///   },
  ///   body: new Uint8Array([1, 2, 3, 4])
  /// });
  /// ```
  #[napi(constructor)]
  pub fn constructor(options: PhpRequestOptions) -> Result<Self> {
    let mut builder: RequestBuilder = Request::builder().method(options.method).url(&options.url);

    if let Some(socket) = options.socket {
      let local_socket = format!("{}:{}", socket.local_address, socket.local_port);
      let remote_socket = format!("{}:{}", socket.remote_address, socket.remote_port);

      builder = builder
        .local_socket(&local_socket)
        .remote_socket(&remote_socket);
    }

    if let Some(headers) = options.headers {
      for key in headers.keys() {
        let values = headers.get(key).ok_or_else(|| {
          Error::from_reason(format!("Missing header values for key \"{}\"", key))
        })?;

        for value in values {
          builder = builder.header(key.clone(), value.clone())
        }
      }
    }

    if let Some(body) = options.body {
      builder = builder.body(body.as_ref())
    }

    Ok(PhpRequest {
      request: builder
        .build()
        .map_err(|err| Error::from_reason(err.to_string()))?,
    })
  }

  /// Get the HTTP method for the request.
  ///
  /// # Examples
  ///
  /// ```js
  /// const request = new Request({
  ///   method: 'GET'
  /// });
  ///
  /// console.log(request.method);
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn method(&self) -> String {
    self.request.method().to_owned()
  }

  /// Get the URL for the request.
  ///
  /// # Examples
  ///
  /// ```js
  /// const request = new Request({
  ///   url: 'http://example.com'
  /// });
  ///
  /// console.log(request.url);
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn url(&self) -> String {
    self.request.url().as_str().to_owned()
  }

  /// Get the headers for the request.
  ///
  /// # Examples
  ///
  /// ```js
  /// const request = new Request({
  ///   headers: {
  ///     'Accept': ['application/json', 'text/html']
  ///   }
  /// });
  ///
  /// for (const mime of request.headers.get('Accept')) {
  ///   console.log(mime);
  /// }
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn headers(&self) -> PhpHeaders {
    PhpHeaders::new(self.request.headers().clone())
  }

  /// Get the body for the request.
  ///
  /// # Examples
  ///
  /// ```js
  /// const request = new Request({
  ///   body: new Uint8Array([1, 2, 3, 4])
  /// });
  ///
  /// console.log(request.body);
  /// ```
  #[napi(getter, enumerable = true)]
  pub fn body(&self) -> Buffer {
    self.request.body().to_vec().into()
  }
}

impl From<&PhpRequest> for Request {
  fn from(request: &PhpRequest) -> Self {
    request.request.clone()
  }
}

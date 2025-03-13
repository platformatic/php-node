use std::collections::HashMap;

use napi::bindgen_prelude::*;

use php::{Request, RequestBuilder};

use crate::PhpHeaders;

/// Options for creating a new PHP request.
#[napi(object)]
#[derive(Default)]
pub struct PhpRequestOptions {
    /// The HTTP method for the request.
    pub method: String,
    /// The URL for the request.
    pub url: String,
    /// The headers for the request.
    pub headers: Option<HashMap<String, Vec<String>>>,
    /// The body for the request.
    pub body: Option<Uint8Array>
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
    request: Request
}

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
    pub fn constructor(options: PhpRequestOptions) -> Self {
        let mut builder: RequestBuilder = Request::builder()
            .method(options.method)
            .url(options.url).expect("invalid url");

        if let Some(headers) = options.headers {
            for key in headers.keys() {
                let values = headers.get(key)
                    .expect(format!("missing header values for key: {}", key).as_str());

                for value in values {
                    builder = builder.header(key.clone(), value.clone())
                }
            }
        }

        if let Some(body) = options.body {
            builder = builder.body(body.as_ref())
        }

        PhpRequest {
            request: builder.build()
        }
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
        self.request
            .url()
            .as_str()
            .to_owned()
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
        self.request
            .body()
            .to_vec()
            .into()
    }
}

impl From<&PhpRequest> for Request {
    fn from(request: &PhpRequest) -> Self {
        request.request.clone()
    }
}

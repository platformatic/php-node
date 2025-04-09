use std::collections::HashMap;

use napi::bindgen_prelude::*;

use php::Response;

use crate::PhpHeaders;

/// Options for creating a new PHP response.
#[napi(object)]
pub struct PhpResponseOptions {
    /// The HTTP status code for the response.
    pub status: i32,
    /// The headers for the response.
    /// TODO: Figure out how to accept a Headers instance
    /// TODO: Figure out how to support both single values without array wrap
    pub headers: Option<HashMap<String, Vec<String>>>,
    /// The body for the response.
    pub body: Option<Uint8Array>,
    /// The log for the response.
    pub log: Option<Uint8Array>,
    /// The exception for the response.
    pub exception: Option<String>
}

/// A PHP response.
#[napi(js_name = "Response")]
pub struct PhpResponse {
    response: Response
}

impl PhpResponse {
    // Create a new PHP response instance.
    pub fn new(response: Response) -> Self {
        PhpResponse {
            response
        }
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
    pub fn constructor(options: PhpResponseOptions) -> Self {
        let mut builder = Response::builder();
        builder.status(options.status);

        if let Some(headers) = options.headers {
            for key in headers.keys() {
                let values = headers.get(key)
                    .expect(format!("missing header values for key: {}", key).as_str());

                for value in values {
                    builder.header(key.clone(), value.clone());
                }
            }
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

        PhpResponse {
            response: builder.build()
        }
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
        self.response
            .body()
            .to_vec()
            .into()
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
        self.response
            .log()
            .to_vec()
            .into()
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

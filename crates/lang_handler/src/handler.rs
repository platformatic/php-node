use super::{Request, Response};

/// Enables a type to support handling HTTP requests.
///
/// # Example
///
/// ```
/// use lang_handler::{Handler, Request, Response, ResponseBuilder};
///
/// struct MyHandler;
///
/// impl Handler for MyHandler {
///   type Error = String;
///
///   fn handle(&self, request: Request) -> Result<Response, Self::Error> {
///     let response = Response::builder()
///       .status(200)
///       .header("Content-Type", "text/plain")
///       .body(request.body())
///       .build();
///
///     Ok(response)
///   }
/// }
pub trait Handler {
  type Error;

  /// Handles an HTTP request.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::{Handler, Request, Response};
  ///
  /// # struct MyHandler;
  /// # impl Handler for MyHandler {
  /// #   type Error = String;
  /// #
  /// #   fn handle(&self, request: Request) -> Result<Response, Self::Error> {
  /// #     let response = Response::builder()
  /// #       .status(200)
  /// #       .header("Content-Type", "text/plain")
  /// #       .body(request.body())
  /// #       .build();
  /// #
  /// #     Ok(response)
  /// #   }
  /// # }
  /// # let handler = MyHandler;
  /// #
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let response = handler.handle(request).unwrap();
  /// ```
  fn handle(&self, request: Request) -> Result<Response, Self::Error>;
}

use super::{Request, Rewriter};

impl<F> Rewriter for F
where
  F: Fn(Request) -> Request + Sync + Send,
{
  /// Rewrites the request by calling the Fn(&Request) with the given request
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{Request, rewrite::Rewriter};
  /// let rewriter = |request: Request| {
  ///   request.extend()
  ///     .url("http://example.com/foo/bar")
  ///     .build()
  ///     .expect("should build new request")
  /// };
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("request should build");
  ///
  /// assert_eq!(
  ///   rewriter.rewrite(request).url().path(),
  ///   "/foo/bar".to_string()
  /// );
  /// ```
  fn rewrite(&self, request: Request) -> Request {
    self(request)
  }
}

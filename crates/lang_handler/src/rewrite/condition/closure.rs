use super::{Condition, Request};

impl<F> Condition for F
where
  F: Fn(&Request) -> bool + Sync + Send,
{
  /// Matches if calling the Fn(&Request) with the given request returns true
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{Request, rewrite::Condition};
  /// let condition = |request: &Request| {
  ///   request.url().path().contains("/foo")
  /// };
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("request should build");
  ///
  /// assert_eq!(condition.matches(&request), false);
  /// ```
  fn matches(&self, request: &Request) -> bool {
    self(request)
  }
}

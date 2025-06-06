use std::path::Path;

use super::{Condition, Request};

impl<F> Condition for F
where
  F: Fn(&Request, &Path) -> bool + Sync + Send,
{
  /// Matches if calling the Fn(&Request) with the given request returns true
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::path::Path;
  /// # use lang_handler::{Request, rewrite::Condition};
  /// # let docroot = std::env::temp_dir();
  /// let condition = |request: &Request, _docroot: &Path| -> bool {
  ///   request.url().path().contains("/foo")
  /// };
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("request should build");
  ///
  /// assert!(!condition.matches(&request, &docroot));
  /// ```
  fn matches(&self, request: &Request, docroot: &Path) -> bool {
    self(request, docroot)
  }
}

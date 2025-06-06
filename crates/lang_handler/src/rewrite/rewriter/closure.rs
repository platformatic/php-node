use std::path::Path;

use super::{Request, RequestBuilderException, Rewriter};

impl<F> Rewriter for F
where
  F: Fn(Request, &Path) -> Result<Request, RequestBuilderException> + Sync + Send,
{
  /// Rewrites the request by calling the Fn(&Request) with the given request
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::path::Path;
  /// # use lang_handler::{Request, rewrite::Rewriter};
  /// # let docroot = std::env::temp_dir();
  /// let rewriter = |request: Request, docroot: &Path| {
  ///   request.extend()
  ///     .url("http://example.com/foo/bar")
  ///     .build()
  /// };
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("request should build");
  ///
  /// let new_request = rewriter.rewrite(request, &docroot)
  ///   .expect("rewriting should succeed");
  ///
  /// assert_eq!(new_request.url().path(), "/foo/bar".to_string());
  /// ```
  fn rewrite(&self, request: Request, docroot: &Path) -> Result<Request, RequestBuilderException> {
    self(request, docroot)
  }
}

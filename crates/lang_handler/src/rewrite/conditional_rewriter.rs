use std::path::Path;

use crate::{
  rewrite::{Condition, Rewriter},
  Request, RequestBuilderException,
};

// Tested via Rewriter::when(...) doc-test

/// This provides a rewriter that applies another rewriter conditionally based
/// on a condition.
pub struct ConditionalRewriter<R, C>(Box<R>, Box<C>)
where
  R: Rewriter + ?Sized,
  C: Condition + ?Sized;

impl<R, C> ConditionalRewriter<R, C>
where
  R: Rewriter + ?Sized,
  C: Condition + ?Sized,
{
  /// Constructs a new ConditionalRewriter with the given rewriter and
  /// condition. The rewriter will only be applied if the condition matches
  /// the request.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lang_handler::rewrite::{
  /// #   Rewriter,
  /// #   ConditionalRewriter,
  /// #   PathCondition,
  /// #   PathRewriter
  /// # };
  /// let condition = PathCondition::new("^/index\\.php$")
  ///   .expect("should be valid regex");
  ///
  /// let rewriter = PathRewriter::new("^(.*)$", "/foo$1")
  ///   .expect("should be valid regex");
  ///
  /// let conditional_rewriter =
  ///   ConditionalRewriter::new(rewriter, condition);
  /// ```
  pub fn new(rewriter: Box<R>, condition: Box<C>) -> Box<Self> {
    Box::new(Self(rewriter, condition))
  }
}

impl<R, C> Rewriter for ConditionalRewriter<R, C>
where
  R: Rewriter + ?Sized,
  C: Condition + ?Sized,
{
  /// A ConditionalRewriter matches a request if its condition matches the
  /// request. If it does, the rewriter is applied to the request.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lang_handler::{
  /// #   Request,
  /// #   rewrite::{
  /// #     Condition,
  /// #     ConditionalRewriter,
  /// #     PathCondition,
  /// #     PathRewriter,
  /// #     Rewriter
  /// #   }
  /// # };
  /// # let docroot = std::env::temp_dir();
  /// let condition = PathCondition::new("^/index\\.php$")
  ///   .expect("should be valid regex");
  ///
  /// let rewriter = PathRewriter::new("^(.*)$", "/foo$1")
  ///   .expect("should be valid regex");
  ///
  /// let conditional_rewriter =
  ///   ConditionalRewriter::new(rewriter, condition);
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let new_request = conditional_rewriter.rewrite(request, &docroot)
  ///   .expect("should rewrite request");
  ///
  /// assert_eq!(new_request.url().path(), "/foo/index.php".to_string());
  /// #
  /// # let request = Request::builder()
  /// #   .url("http://example.com/other.php")
  /// #   .build()
  /// #   .expect("should build request");
  /// #
  /// # let new_request = conditional_rewriter.rewrite(request, &docroot)
  /// #   .expect("should rewrite request");
  /// #
  /// # assert_eq!(new_request.url().path(), "/other.php".to_string());
  /// ```
  fn rewrite(&self, request: Request, docroot: &Path) -> Result<Request, RequestBuilderException> {
    if !self.1.matches(&request, docroot) {
      return Ok(request);
    }

    self.0.rewrite(request, docroot)
  }
}

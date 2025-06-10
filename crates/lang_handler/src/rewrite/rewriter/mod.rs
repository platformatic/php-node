use std::path::Path;

use crate::{
  rewrite::{Condition, ConditionalRewriter},
  Request, RequestBuilderException,
};

mod closure;
mod header;
mod href;
mod method;
mod path;
mod sequence;

pub use header::HeaderRewriter;
pub use href::HrefRewriter;
pub use method::MethodRewriter;
pub use path::PathRewriter;
pub use sequence::RewriterSequence;

/// A Rewriter simply applies its rewrite function to produce a possibly new
/// request object.
pub trait Rewriter: Sync + Send {
  /// Rewrite a request using the rewriter's logic.
  fn rewrite(&self, request: Request, docroot: &Path) -> Result<Request, RequestBuilderException>;
}

impl<T: ?Sized> RewriterExt for T where T: Rewriter {}

/// Extends Rewriter with combinators like `when` and `then`.
pub trait RewriterExt: Rewriter {
  /// Add a condition to a rewriter to make it apply conditionally
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{
  /// #  Request,
  /// #  rewrite::{Rewriter, RewriterExt, PathCondition, PathRewriter}
  /// # };
  /// # let docroot = std::env::temp_dir();
  /// let rewriter = PathRewriter::new("^(/index\\.php)$", "/foo$1")
  ///   .expect("should be valid regex");
  ///
  /// let condition = PathCondition::new("^/index\\.php$")
  ///   .expect("should be valid regex");
  ///
  /// let conditional_rewriter = rewriter.when(condition);
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
  /// ```
  fn when<C>(self: Box<Self>, condition: Box<C>) -> Box<ConditionalRewriter<Self, C>>
  where
    C: Condition + ?Sized,
  {
    ConditionalRewriter::new(self, condition)
  }

  /// Add a rewriter to be applied in sequence.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{
  /// #   Request,
  /// #   rewrite::{Rewriter, RewriterExt, PathRewriter, HeaderRewriter}
  /// # };
  /// # let docroot = std::env::temp_dir();
  /// let first = PathRewriter::new("^(/index.php)$", "/foo$1")
  ///   .expect("should be valid regex");
  ///
  /// let second = PathRewriter::new("foo/index", "foo/bar")
  ///   .expect("should be valid regex");
  ///
  /// let sequence = first.then(second);
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .header("TEST", "foo")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let new_request = sequence.rewrite(request, &docroot)
  ///   .expect("should rewrite request");
  ///
  /// assert_eq!(new_request.url().path(), "/foo/bar.php".to_string());
  /// ```
  fn then<R>(self: Box<Self>, rewriter: Box<R>) -> Box<RewriterSequence<Self, R>>
  where
    R: Rewriter + ?Sized,
  {
    RewriterSequence::new(self, rewriter)
  }
}

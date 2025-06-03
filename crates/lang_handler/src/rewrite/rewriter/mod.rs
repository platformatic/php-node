use crate::{
  rewrite::{Condition, ConditionalRewriter},
  Request,
};

mod closure;
mod header;
mod path;
mod sequence;

pub use header::HeaderRewriter;
pub use path::PathRewriter;
pub use sequence::RewriterSequence;

/// A Rewriter simply applies its rewrite function to produce a possibly new
/// request object.
pub trait Rewriter: Sync + Send {
  fn rewrite(&self, request: Request) -> Request;
}

impl<T: ?Sized> RewriterExt for T where T: Rewriter {}

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
  /// assert_eq!(
  ///   conditional_rewriter.rewrite(request).url().path(),
  ///   "/foo/index.php".to_string()
  /// );
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
  /// assert_eq!(
  ///   sequence.rewrite(request).url().path(),
  ///   "/foo/bar.php".to_string()
  /// );
  /// ```
  fn then<R>(self: Box<Self>, rewriter: Box<R>) -> Box<RewriterSequence<Self, R>>
  where
    R: Rewriter + ?Sized,
  {
    RewriterSequence::new(self, rewriter)
  }
}

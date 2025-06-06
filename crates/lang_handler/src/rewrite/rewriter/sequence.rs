use std::path::Path;

use super::{Request, RequestBuilderException, Rewriter};

// Tested via Rewriter::then(...) doc-test

/// This provides sequencing of rewriters.
pub struct RewriterSequence<A, B>(Box<A>, Box<B>)
where
  A: Rewriter + ?Sized,
  B: Rewriter + ?Sized;

impl<A, B> RewriterSequence<A, B>
where
  A: Rewriter + ?Sized,
  B: Rewriter + ?Sized,
{
  /// Constructs a new RewriterSequence with the given rewriters applied in
  /// sequence.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lang_handler::rewrite::{Rewriter, RewriterSequence, PathRewriter};
  /// let first = PathRewriter::new("^(.*)$", "/bar$1")
  ///   .expect("should be valid regex");
  ///
  /// let second = PathRewriter::new("^(.*)$", "/foo$1")
  ///   .expect("should be valid regex");
  ///
  /// let sequence = RewriterSequence::new(first, second);
  /// ```
  pub fn new(a: Box<A>, b: Box<B>) -> Box<Self> {
    Box::new(Self(a, b))
  }
}

impl<A, B> Rewriter for RewriterSequence<A, B>
where
  A: Rewriter + ?Sized,
  B: Rewriter + ?Sized,
{
  /// Rewrite a request using the first rewriter, then the second.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use std::path::Path;
  /// # use lang_handler::{
  /// #   Request,
  /// #   rewrite::{Rewriter, RewriterSequence, PathRewriter}
  /// # };
  /// # let docroot = std::env::temp_dir();
  /// let first = PathRewriter::new("^(.*)$", "/bar$1")
  ///   .expect("should be valid regex");
  ///
  /// let second = PathRewriter::new("^(.*)$", "/foo$1")
  ///   .expect("should be valid regex");
  ///
  /// let sequence = RewriterSequence::new(first, second);
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let new_request = sequence.rewrite(request, &docroot)
  ///   .expect("should rewrite request");
  ///
  /// assert_eq!(new_request.url().path(), "/foo/bar/index.php".to_string());
  /// ```
  fn rewrite(&self, request: Request, docroot: &Path) -> Result<Request, RequestBuilderException> {
    let request = self.0.rewrite(request, docroot)?;
    self.1.rewrite(request, docroot)
  }
}

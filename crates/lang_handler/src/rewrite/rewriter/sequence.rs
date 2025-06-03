use super::{Request, Rewriter};

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
  pub fn new(a: Box<A>, b: Box<B>) -> Box<Self> {
    Box::new(Self(a, b))
  }
}

impl<A, B> Rewriter for RewriterSequence<A, B>
where
  A: Rewriter + ?Sized,
  B: Rewriter + ?Sized,
{
  fn rewrite(&self, request: Request) -> Request {
    let request = self.0.rewrite(request);
    self.1.rewrite(request)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::rewrite::{PathRewriter, RewriterExt};
  use crate::Request;

  #[test]
  fn test_rewrite_sequence() {
    let first = PathRewriter::new("^(/index.php)$", "/foo$1").expect("should be valid regex");

    let second = PathRewriter::new("foo/index", "foo/bar").expect("should be valid regex");

    let sequence = first.then(second);

    let request = Request::builder()
      .url("http://example.com/index.php")
      .header("TEST", "foo")
      .build()
      .expect("should build request");

    assert_eq!(
      sequence.rewrite(request).url().path(),
      "/foo/bar.php".to_string()
    );
  }
}

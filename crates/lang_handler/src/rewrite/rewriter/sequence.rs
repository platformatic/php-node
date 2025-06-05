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

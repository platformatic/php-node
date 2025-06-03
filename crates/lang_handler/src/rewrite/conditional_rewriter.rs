use crate::{
  rewrite::{Condition, Rewriter},
  Request,
};

// Tested via Rewriter::when(...) doc-test

pub struct ConditionalRewriter<R, C>(Box<R>, Box<C>)
where
  R: Rewriter + ?Sized,
  C: Condition + ?Sized;

impl<R, C> ConditionalRewriter<R, C>
where
  R: Rewriter + ?Sized,
  C: Condition + ?Sized,
{
  pub fn new(rewriter: Box<R>, condition: Box<C>) -> Box<Self> {
    Box::new(Self(rewriter, condition))
  }
}

impl<R, C> Condition for ConditionalRewriter<R, C>
where
  R: Rewriter + ?Sized,
  C: Condition + ?Sized,
{
  fn matches(&self, request: &Request) -> bool {
    self.1.matches(request)
  }
}

impl<R, C> Rewriter for ConditionalRewriter<R, C>
where
  R: Rewriter + ?Sized,
  C: Condition + ?Sized,
{
  fn rewrite(&self, request: Request) -> Request {
    self
      .matches(&request)
      .then(|| self.0.rewrite(request.clone()))
      .or(Some(request))
      .expect("should produce a request")
  }
}

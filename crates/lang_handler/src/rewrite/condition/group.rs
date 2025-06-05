use super::{Condition, Request};

// Tested via Condition::and(...) and Condition::or(...) doctests

/// This provides logical grouping of conditions using either AND or OR
/// combination behaviours.
pub enum ConditionGroup<A, B>
where
  A: Condition + ?Sized,
  B: Condition + ?Sized,
{
  Or(Box<A>, Box<B>),
  And(Box<A>, Box<B>),
}

impl<A, B> ConditionGroup<A, B>
where
  A: Condition + ?Sized,
  B: Condition + ?Sized,
{
  pub fn and(a: Box<A>, b: Box<B>) -> Box<Self> {
    Box::new(ConditionGroup::And(a, b))
  }

  pub fn or(a: Box<A>, b: Box<B>) -> Box<Self> {
    Box::new(ConditionGroup::Or(a, b))
  }
}

impl<A, B> Condition for ConditionGroup<A, B>
where
  A: Condition + ?Sized,
  B: Condition + ?Sized,
{
  fn matches(&self, request: &Request) -> bool {
    match self {
      ConditionGroup::Or(a, b) => a.matches(request) || b.matches(request),
      ConditionGroup::And(a, b) => a.matches(request) && b.matches(request),
    }
  }
}

use super::{Condition, Request};

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

#[cfg(test)]
mod test {
  use super::*;
  use crate::rewrite::{ConditionExt, HeaderCondition, PathCondition};

  #[test]
  fn test_condition_group_and() {
    let header = HeaderCondition::new("TEST", "^foo$").expect("should be valid regex");

    let path = PathCondition::new("^/index\\.php$").expect("should be valid regex");

    let header_and_path = header.and(path);

    // Check it matches when all conditions match
    let request = Request::builder()
      .url("http://example.com/index.php")
      .header("TEST", "foo")
      .build()
      .expect("request should build");

    assert!(header_and_path.matches(&request));

    // Check it _does not_ match if either condition does not match
    let only_header = Request::builder()
      .url("http://example.com/nope.php")
      .header("TEST", "foo")
      .build()
      .expect("request should build");

    assert!(!header_and_path.matches(&only_header));

    let only_url = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("request should build");

    assert!(!header_and_path.matches(&only_url));
  }

  #[test]
  fn test_condition_group_or() {
    let header = HeaderCondition::new("TEST", "^foo$").expect("should be valid regex");

    let path = PathCondition::new("^/index\\.php$").expect("should be valid regex");

    let header_or_path = header.or(path);

    // Check it matches when either condition matches
    let request = Request::builder()
      .url("http://example.com/index.php")
      .header("TEST", "foo")
      .build()
      .expect("request should build");

    assert!(header_or_path.matches(&request));

    // Check it matches if either condition does not match
    let only_header = Request::builder()
      .url("http://example.com/nope.php")
      .header("TEST", "foo")
      .build()
      .expect("request should build");

    assert!(header_or_path.matches(&only_header));

    let only_url = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("request should build");

    assert!(header_or_path.matches(&only_url));
  }
}

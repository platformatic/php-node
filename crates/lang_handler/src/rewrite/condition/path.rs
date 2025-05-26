use std::fmt::Debug;

use regex::{Error, Regex};

use super::Condition;
use super::Request;

/// Match request path to a regex pattern
#[derive(Clone, Debug)]
pub struct PathCondition {
  pattern: Regex,
}

impl PathCondition {
  /// Construct a new PathCondition matching the given Regex pattern.
  pub fn new<R>(pattern: R) -> Result<Self, Error>
  where
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
  {
    let pattern = pattern.try_into()?;
    Ok(Self { pattern })
  }
}

impl Condition for PathCondition {
  /// A PathCondition matches a request if the path segment of the request url
  /// matches the pattern given when constructing the PathCondition.
  fn matches(&self, request: &Request) -> bool {
    self.pattern.is_match(request.url().path())
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_path_condition() {
    let condition = PathCondition::new("^/index.php$").expect("regex should be valid");

    let request = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("request should build");

    assert!(condition.matches(&request));
  }
}

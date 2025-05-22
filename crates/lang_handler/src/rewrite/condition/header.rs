use std::fmt::Debug;

use regex::{Error, Regex};

use super::Condition;
use crate::Request;

/// Match request header to a regex pattern
#[derive(Clone, Debug)]
pub struct HeaderCondition {
  name: String,
  pattern: Regex,
}

impl HeaderCondition {
  /// Construct a new HeaderCondition matching the given header name and Regex
  /// pattern.
  pub fn new<S, R>(name: S, pattern: R) -> Result<Self, Error>
  where
    S: Into<String>,
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
  {
    let name = name.into();
    let pattern = pattern.try_into()?;
    Ok(Self { name, pattern })
  }
}

impl Condition for HeaderCondition {
  /// A HeaderCondition matches a given request if the header specified in the
  /// constructor is both present and matches the given Regex pattern.
  fn matches(&self, request: &Request) -> bool {
    request
      .headers()
      .get_line(&self.name)
      .map(|line| self.pattern.is_match(&line))
      .unwrap_or(false)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_header_condition() {
    let condition = HeaderCondition::new("TEST", "^foo$").expect("regex should be valid");

    let request = Request::builder()
      .url("http://example.com/")
      .header("TEST", "foo")
      .build()
      .expect("request should build");

    assert!(condition.matches(&request));
  }
}

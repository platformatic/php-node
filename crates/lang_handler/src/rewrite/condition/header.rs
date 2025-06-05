use std::fmt::Debug;

use regex::{Error, Regex};

use super::Condition;
use crate::Request;

/// Matches a request header to a regex pattern
#[derive(Clone, Debug)]
pub struct HeaderCondition {
  name: String,
  pattern: Regex,
}

impl HeaderCondition {
  /// Construct a new HeaderCondition matching the given header name and Regex
  /// pattern.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, HeaderCondition};
  /// # use lang_handler::Request;
  /// let condition = HeaderCondition::new("TEST", "^foo$")
  ///   .expect("should be valid regex");
  /// ```
  pub fn new<S, R>(name: S, pattern: R) -> Result<Box<Self>, Error>
  where
    S: Into<String>,
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
  {
    let name = name.into();
    let pattern = pattern.try_into()?;
    Ok(Box::new(Self { name, pattern }))
  }
}

impl Condition for HeaderCondition {
  /// A HeaderCondition matches a given request if the header specified in the
  /// constructor is both present and matches the given Regex pattern.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, HeaderCondition};
  /// # use lang_handler::Request;
  /// let condition = HeaderCondition::new("TEST", "^foo$")
  ///   .expect("should be valid regex");
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .header("TEST", "foo")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request));
  /// ```
  fn matches(&self, request: &Request) -> bool {
    request
      .headers()
      .get_line(&self.name)
      .map(|line| self.pattern.is_match(&line))
      .unwrap_or(false)
  }
}

use std::{fmt::Debug, path::Path};

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
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::PathCondition;
  /// let condition = PathCondition::new("^/index.php$")
  ///   .expect("should be valid regex");
  /// ```
  pub fn new<R>(pattern: R) -> Result<Box<Self>, Error>
  where
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
  {
    let pattern = pattern.try_into()?;
    Ok(Box::new(Self { pattern }))
  }
}

impl Condition for PathCondition {
  /// A PathCondition matches a request if the path segment of the request url
  /// matches the pattern given when constructing the PathCondition.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, PathCondition};
  /// # use lang_handler::Request;
  /// # let docroot = std::env::temp_dir();
  /// let condition = PathCondition::new("^/index.php$")
  ///   .expect("should be valid regex");
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// # assert!(!condition.matches(
  /// #   &request.extend()
  /// #     .url("http://example.com/other.php")
  /// #     .build()
  /// #     .expect("should build request"),
  /// #   &docroot
  /// # ));
  /// ```
  fn matches(&self, request: &Request, _docroot: &Path) -> bool {
    self.pattern.is_match(request.url().path())
  }
}

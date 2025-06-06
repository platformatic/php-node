use std::{fmt::Debug, path::Path};

use regex::{Error, Regex};

use super::Condition;
use crate::Request;

/// Matches a request method to a regex pattern
#[derive(Clone, Debug)]
pub struct MethodCondition(Regex);

impl MethodCondition {
  /// Construct a new MethodCondition matching the Request method to the given
  /// Regex pattern.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, MethodCondition};
  /// # use lang_handler::Request;
  /// let condition = MethodCondition::new("GET")
  ///   .expect("should be valid regex");
  /// ```
  pub fn new<R>(pattern: R) -> Result<Box<Self>, Error>
  where
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
  {
    let pattern = pattern.try_into()?;
    Ok(Box::new(Self(pattern)))
  }
}

impl Condition for MethodCondition {
  /// A MethodCondition matches a given request if the Request method matches
  /// the given Regex pattern.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, MethodCondition};
  /// # use lang_handler::Request;
  /// # let docroot = std::env::temp_dir();
  /// let condition = MethodCondition::new("GET")
  ///   .expect("should be valid regex");
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// # assert!(!condition.matches(
  /// #   &request.extend()
  /// #     .method("POST")
  /// #     .build()
  /// #     .expect("should build request"),
  /// #   &docroot
  /// # ));
  /// ```
  fn matches(&self, request: &Request, _docroot: &Path) -> bool {
    self.0.is_match(request.method())
  }
}

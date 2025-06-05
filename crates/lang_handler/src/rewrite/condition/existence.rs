use std::path::PathBuf;

use super::Condition;
use super::Request;

/// Match if request path exists
#[derive(Clone, Debug)]
pub struct ExistenceCondition(PathBuf);

impl ExistenceCondition {
  /// Construct an ExistenceCondition to check within a given base directory.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, ExistenceCondition};
  /// # use lang_handler::Request;
  /// let condition = ExistenceCondition::new("/foo/bar");
  /// ```
  pub fn new<P>(base: P) -> Self
  where
    P: Into<PathBuf>,
  {
    Self(base.into())
  }
}

impl Condition for ExistenceCondition {
  /// An ExistenceCondition matches a request if the path segment of the
  /// request url exists in the provided base directory.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, ExistenceCondition};
  /// # use lang_handler::Request;
  /// let condition = ExistenceCondition::new("/foo/bar");
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(condition.matches(&request), false);
  /// ```
  fn matches(&self, request: &Request) -> bool {
    let path = request.url().path();
    self
      .0
      .join(path.strip_prefix("/").unwrap_or(path))
      .canonicalize()
      .is_ok()
  }
}

/// Match if request path does not exist
#[derive(Clone, Debug, Default)]
pub struct NonExistenceCondition(PathBuf);

impl NonExistenceCondition {
  /// Construct a NonExistenceCondition to check within a given base directory.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, NonExistenceCondition};
  /// # use lang_handler::Request;
  /// let condition = NonExistenceCondition::new("/foo/bar");
  /// ```
  pub fn new<P>(base: P) -> Self
  where
    P: Into<PathBuf>,
  {
    Self(base.into())
  }
}

impl Condition for NonExistenceCondition {
  /// A NonExistenceCondition matches a request if the path segment of the
  /// request url does not exist in the provided base directory.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Condition, NonExistenceCondition};
  /// # use lang_handler::Request;
  /// let condition = NonExistenceCondition::new("/foo/bar");
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request));
  /// ```
  fn matches(&self, request: &Request) -> bool {
    let path = request.url().path();
    self
      .0
      .join(path.strip_prefix("/").unwrap_or(path))
      .canonicalize()
      .is_err()
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::MockRoot;

  #[test]
  fn test_existence_condition() {
    let docroot = MockRoot::builder()
      .file("exists.php", "<?php echo \"Hello, world!\"; ?>")
      .build()
      .expect("should prepare docroot");

    let condition = ExistenceCondition::new(docroot.clone());

    let request = Request::builder()
      .url("http://example.com/exists.php")
      .build()
      .expect("request should build");

    assert!(condition.matches(&request));
  }

  #[test]
  fn test_non_existence_condition() {
    let docroot = MockRoot::builder().build().expect("should prepare docroot");

    let condition = NonExistenceCondition::new(docroot.clone());

    let request = Request::builder()
      .url("http://example.com/does_not_exist.php")
      .build()
      .expect("request should build");

    assert!(condition.matches(&request));
  }
}

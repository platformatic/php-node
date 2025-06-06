use std::path::Path;

use super::Condition;
use super::Request;

/// Match if request path exists
#[derive(Clone, Debug, Default)]
pub struct ExistenceCondition;

impl Condition for ExistenceCondition {
  /// An ExistenceCondition matches a request if the path segment of the
  /// request url exists in the provided base directory.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{
  /// #   rewrite::{Condition, ExistenceCondition},
  /// #   Request,
  /// #   MockRoot
  /// # };
  /// #
  /// # let docroot = MockRoot::builder()
  /// #   .file("exists.php", "<?php echo \"Hello, world!\"; ?>")
  /// #   .build()
  /// #   .expect("should prepare docroot");
  /// let condition = ExistenceCondition;
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/exists.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// # assert!(!condition.matches(
  /// #   &request.extend()
  /// #      .url("http://example.com/does_not_exist.php")
  /// #      .build()
  /// #      .expect("should build request"),
  /// #   &docroot
  /// # ));
  /// ```
  fn matches(&self, request: &Request, docroot: &Path) -> bool {
    let path = request.url().path();
    docroot
      .join(path.strip_prefix("/").unwrap_or(path))
      .canonicalize()
      .is_ok()
  }
}

/// Match if request path does not exist
#[derive(Clone, Debug, Default)]
pub struct NonExistenceCondition;

impl Condition for NonExistenceCondition {
  /// A NonExistenceCondition matches a request if the path segment of the
  /// request url does not exist in the provided base directory.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{
  /// #   rewrite::{Condition, NonExistenceCondition},
  /// #   Request,
  /// #   MockRoot
  /// # };
  /// #
  /// # let docroot = MockRoot::builder()
  /// #   .file("exists.php", "<?php echo \"Hello, world!\"; ?>")
  /// #   .build()
  /// #   .expect("should prepare docroot");
  /// let condition = NonExistenceCondition;
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/does_not_exist.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// # assert!(!condition.matches(
  /// #   &request.extend()
  /// #      .url("http://example.com/exists.php")
  /// #      .build()
  /// #      .expect("should build request"),
  /// #   &docroot
  /// # ));
  /// ```
  fn matches(&self, request: &Request, docroot: &Path) -> bool {
    let path = request.url().path();
    docroot
      .join(path.strip_prefix("/").unwrap_or(path))
      .canonicalize()
      .is_err()
  }
}

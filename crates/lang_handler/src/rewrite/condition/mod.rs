mod closure;
mod existence;
mod group;
mod header;
mod method;
mod path;

use std::path::Path;

use crate::Request;

pub use existence::{ExistenceCondition, NonExistenceCondition};
pub use group::ConditionGroup;
pub use header::HeaderCondition;
pub use method::MethodCondition;
pub use path::PathCondition;

/// A Condition is used to match against request state before deciding to apply
/// a given Rewrite or set of Rewrites.
pub trait Condition: Sync + Send {
  /// A Condition must implement a `matches(request) -> bool` method which
  /// receives a request object to determine if the condition is met.
  fn matches(&self, request: &Request, docroot: &Path) -> bool;
}

impl<T: ?Sized> ConditionExt for T where T: Condition {}

/// Extends Condition with combinators like `and` and `or`.
pub trait ConditionExt: Condition {
  /// Make a new condition which must pass both conditions
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{
  /// #  Request,
  /// #  rewrite::{Condition, ConditionExt, PathCondition, HeaderCondition}
  /// # };
  /// # let docroot = std::env::temp_dir();
  /// let path = PathCondition::new("^/index.php$")
  ///   .expect("should be valid regex");
  ///
  /// let header = HeaderCondition::new("TEST", "^foo$")
  ///   .expect("should be valid regex");
  ///
  /// let condition = path.and(header);
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .header("TEST", "foo")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// #
  /// # // SHould _not_ match if either condition does not match
  /// # let only_header = Request::builder()
  /// #   .url("http://example.com/nope.php")
  /// #   .header("TEST", "foo")
  /// #   .build()
  /// #   .expect("request should build");
  /// #
  /// # assert!(!condition.matches(&only_header, &docroot));
  /// #
  /// # let only_url = Request::builder()
  /// #   .url("http://example.com/index.php")
  /// #   .build()
  /// #   .expect("request should build");
  /// #
  /// # assert!(!condition.matches(&only_url, &docroot));
  /// ```
  fn and<C>(self: Box<Self>, other: Box<C>) -> Box<ConditionGroup<Self, C>>
  where
    C: Condition + ?Sized,
  {
    ConditionGroup::and(self, other)
  }

  /// Make a new condition which must pass either condition
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{
  /// #  Request,
  /// #  rewrite::{Condition, ConditionExt, PathCondition, HeaderCondition}
  /// # };
  /// # let docroot = std::env::temp_dir();
  /// let path = PathCondition::new("^/index.php$")
  ///   .expect("should be valid regex");
  ///
  /// let header = HeaderCondition::new("TEST", "^foo$")
  ///   .expect("should be valid regex");
  ///
  /// let condition = path.or(header);
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// #
  /// # // Should match if one condition does not
  /// # let only_header = Request::builder()
  /// #   .url("http://example.com/nope.php")
  /// #   .header("TEST", "foo")
  /// #   .build()
  /// #   .expect("request should build");
  /// #
  /// # assert!(condition.matches(&only_header, &docroot));
  /// #
  /// # let only_url = Request::builder()
  /// #   .url("http://example.com/index.php")
  /// #   .build()
  /// #   .expect("request should build");
  /// #
  /// # assert!(condition.matches(&only_url, &docroot));
  /// ```
  fn or<C>(self: Box<Self>, other: Box<C>) -> Box<ConditionGroup<Self, C>>
  where
    C: Condition + ?Sized,
  {
    ConditionGroup::or(self, other)
  }
}

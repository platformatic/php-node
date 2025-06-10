use std::path::Path;

use super::{Condition, Request};

// Tested via Condition::and(...) and Condition::or(...) doctests

/// This provides logical grouping of conditions using either AND or OR
/// combination behaviours.
pub enum ConditionGroup<A, B>
where
  A: Condition + ?Sized,
  B: Condition + ?Sized,
{
  /// Combines two conditions using logical OR.
  Or(Box<A>, Box<B>),

  /// Combines two conditions using logical AND.
  And(Box<A>, Box<B>),
}

impl<A, B> ConditionGroup<A, B>
where
  A: Condition + ?Sized,
  B: Condition + ?Sized,
{
  /// Constructs a new ConditionGroup with the given conditions combined using
  /// logical AND.
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::path::Path;
  /// # use lang_handler::{Request, rewrite::{Condition, ConditionGroup}};
  /// # let docroot = std::env::temp_dir();
  /// let condition = ConditionGroup::and(
  ///   Box::new(|_req: &Request, _docroot: &Path| true),
  ///   Box::new(|_req: &Request, _docroot: &Path| false),
  /// );
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(!condition.matches(&request, &docroot));
  /// #
  /// # assert!(ConditionGroup::and(
  /// #   Box::new(|_req: &Request, _docroot: &Path| true),
  /// #   Box::new(|_req: &Request, _docroot: &Path| true),
  /// # ).matches(&request, &docroot));
  /// ```
  pub fn and(a: Box<A>, b: Box<B>) -> Box<Self> {
    Box::new(ConditionGroup::And(a, b))
  }

  /// Constructs a new ConditionGroup with the given conditions combined using
  /// logical OR.
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::path::Path;
  /// # use lang_handler::{Request, rewrite::{Condition, ConditionGroup}};
  /// # let docroot = std::env::temp_dir();
  /// let condition = ConditionGroup::or(
  ///   Box::new(|_req: &Request, _docroot: &Path| true),
  ///   Box::new(|_req: &Request, _docroot: &Path| false),
  /// );
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// #
  /// # assert!(!ConditionGroup::or(
  /// #   Box::new(|_req: &Request, _docroot: &Path| false),
  /// #   Box::new(|_req: &Request, _docroot: &Path| false),
  /// # ).matches(&request, &docroot));
  pub fn or(a: Box<A>, b: Box<B>) -> Box<Self> {
    Box::new(ConditionGroup::Or(a, b))
  }
}

impl<A, B> Condition for ConditionGroup<A, B>
where
  A: Condition + ?Sized,
  B: Condition + ?Sized,
{
  /// Evaluates the condition group against the provided request.
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::path::Path;
  /// # let docroot = std::env::temp_dir();
  /// # use lang_handler::{Request, rewrite::{Condition, ConditionGroup}};
  /// let condition = ConditionGroup::or(
  ///   Box::new(|_req: &Request, _docroot: &Path| true),
  ///   Box::new(|_req: &Request, _docroot: &Path| false),
  /// );
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert!(condition.matches(&request, &docroot));
  /// # assert!(!ConditionGroup::or(
  /// #   Box::new(|_req: &Request, _docroot: &Path| false),
  /// #   Box::new(|_req: &Request, _docroot: &Path| false),
  /// # ).matches(&request, &docroot));
  /// ```
  fn matches(&self, request: &Request, docroot: &Path) -> bool {
    match self {
      ConditionGroup::Or(a, b) => a.matches(request, docroot) || b.matches(request, docroot),
      ConditionGroup::And(a, b) => a.matches(request, docroot) && b.matches(request, docroot),
    }
  }
}

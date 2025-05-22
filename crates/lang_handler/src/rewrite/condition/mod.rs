mod header;
mod path;
mod set;

pub use header::*;
pub use path::*;
pub use set::*;

use super::Request;

///
/// Conditions
///

/// A Condition is used to match against request state before deciding to apply
/// a given Rewrite or set of Rewrites.
pub trait Condition: Sync + Send {
  /// A Condition must implement a `matches(request) -> bool` method which
  /// receives a request object to determine if the condition is met.
  fn matches(&self, request: &Request) -> bool;
}

/// Support plain functions as conditions
impl<F> Condition for F
where
  F: Fn(&Request) -> bool + Sync + Send,
{
  fn matches(&self, request: &Request) -> bool {
    self(request)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_closure_condition() {
    let condition = |_: &Request| true;

    let request = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("request should build");

    assert!(condition.matches(&request));
  }
}

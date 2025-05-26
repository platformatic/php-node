use std::fmt::Debug;
use std::hash::Hash;
use std::str::FromStr;

use super::Condition;
use crate::Request;

/// Defines if a set of conditions should match with AND or OR logic
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConditionOperation {
  And,
  Or,
}

impl FromStr for ConditionOperation {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "and" => Ok(ConditionOperation::And),
      "or" => Ok(ConditionOperation::Or),
      _ => Err(format!("Unknown condition operation: {}", s)),
    }
  }
}

/// A set of conditions which may apply together with AND or OR logic
pub struct ConditionSet {
  operation: ConditionOperation,
  conditions: Vec<Box<dyn Condition>>,
}

impl ConditionSet {
  /// Construct a new ConditionSet combining multiple Condition checks into
  /// one set using either AND or OR logic between them.
  pub fn new(operation: ConditionOperation) -> Self {
    Self {
      operation,
      conditions: vec![],
    }
  }

  pub fn change_operation(&mut self, operation: ConditionOperation) {
    self.operation = operation;
  }

  pub fn add_condition(&mut self, condition: Box<dyn Condition>) {
    self.conditions.push(condition);
  }
}

impl Default for ConditionSet {
  /// Default construction of a ConditionSet using AND logic.
  fn default() -> Self {
    Self::new(ConditionOperation::And)
  }
}

impl Condition for ConditionSet {
  /// A ConditionSet matches a given request when:
  ///
  /// - Using AND logic and _all_ conditions in the set match
  /// - Using OR logic and _any_ conditions in the set match
  fn matches(&self, request: &Request) -> bool {
    if self.conditions.len() == 0 {
      true
    } else {
      let mut conds = self.conditions.iter();
      match self.operation {
        ConditionOperation::And => conds.all(|c| c.matches(request)),
        ConditionOperation::Or => conds.any(|c| c.matches(request)),
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::rewrite::{HeaderCondition, PathCondition};

  #[test]
  fn test_condition_set() {
    let mut condition_set = ConditionSet::default();

    let path_condition = PathCondition::new("^/index.php$").expect("regex should be valid");
    condition_set.add_condition(Box::new(path_condition));

    let header_condition = HeaderCondition::new("TEST", "^foo$").expect("regex should be valid");
    condition_set.add_condition(Box::new(header_condition));

    let request = Request::builder()
      .url("http://example.com/index.php")
      .header("TEST", "foo")
      .build()
      .expect("request should build");

    assert!(condition_set.matches(&request));
  }
}

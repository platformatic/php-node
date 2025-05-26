use std::ops::{Deref, DerefMut};

use super::Request;

mod condition;
mod rewriter;

pub use condition::*;
pub use rewriter::*;

///
/// Conditional Rewrite
///

/// Apply a rewrite only if it matches a set of conditions. These conditions
/// may apply using AND or OR logic
pub struct ConditionalRewriter {
  condition_set: ConditionSet,
  rewrite_set: RewriterSet,
}

impl Deref for ConditionalRewriter {
  type Target = ConditionSet;
  fn deref(&self) -> &Self::Target {
    &self.condition_set
  }
}

impl DerefMut for ConditionalRewriter {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.condition_set
  }
}

impl ConditionalRewriter {
  pub fn new(operation: ConditionOperation) -> Self {
    Self {
      condition_set: ConditionSet::new(operation),
      rewrite_set: RewriterSet::default(),
    }
  }

  // TODO: Is there a better way, since multiple-deref is not aalowed?
  pub fn add_rewriter(&mut self, rewrite: Box<dyn Rewriter>) {
    self.rewrite_set.add_rewriter(rewrite);
  }
}

impl Default for ConditionalRewriter {
  fn default() -> Self {
    Self::new(ConditionOperation::And)
  }
}

impl Rewriter for ConditionalRewriter {
  fn rewrite(&self, request: Request) -> Request {
    if self.condition_set.matches(&request) {
      self.rewrite_set.rewrite(request)
    } else {
      request
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_path_rewriting() {
    let mut rewriter = ConditionalRewriter::default();

    let rewrite = PathRewriter::new(r"^/index.php$", "/foo").expect("regex should be valid");

    rewriter.add_rewrite(Box::new(rewrite));

    let request = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("request should build");

    assert_eq!(rewriter.rewrite(request).url().path(), "/foo".to_string());
  }
}

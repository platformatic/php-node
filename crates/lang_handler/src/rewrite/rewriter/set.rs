use super::{Request, Rewriter};

/// A set of Rewrite steps which are applied together in-order.
pub struct RewriterSet {
  rewrites: Vec<Box<dyn Rewriter>>,
}

impl RewriterSet {
  /// Add a Rewrite step to the set.
  pub fn add_rewriter(&mut self, rewrite: Box<dyn Rewriter>) {
    self.rewrites.push(rewrite);
  }
}

impl Default for RewriterSet {
  fn default() -> Self {
    Self { rewrites: vec![] }
  }
}

impl Rewriter for RewriterSet {
  /// Apply each Rewrite in the set in-order.
  fn rewrite(&self, request: Request) -> Request {
    let mut current = request;

    for rewrite in self.rewrites.iter() {
      current = rewrite.rewrite(current);
    }

    current
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::rewrite::PathRewriter;

  #[test]
  fn test_rewriter_set() {
    let mut rewrite_set = RewriterSet::default();

    let first = PathRewriter::new("^(/index.php)$", "/foo$1").expect("regex should be valid");
    rewrite_set.add_rewriter(Box::new(first));

    let second = PathRewriter::new("^/foo", "/bar").expect("regex should be valid");
    rewrite_set.add_rewriter(Box::new(second));

    let request = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("should build request");

    assert_eq!(
      rewrite_set.rewrite(request).url().path(),
      "/bar/index.php".to_string()
    );
  }
}

use super::Request;

mod header;
mod path;
mod set;

pub use header::HeaderRewriter;
pub use path::PathRewriter;
pub use set::RewriterSet;

///
/// Rewriters
///

/// A Rewriter simply applies its rewrite function to produce a possibly new
/// request object.
pub trait Rewriter: Sync + Send {
  fn rewrite(&self, request: Request) -> Request;
}

/// Support plain functions as rewrites
impl<F> Rewriter for F
where
  F: Fn(Request) -> Request + Sync + Send,
{
  fn rewrite(&self, request: Request) -> Request {
    self(request)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_closure_rewriter() {
    let condition = |request: Request| {
      request
        .extend()
        .url("http://example.com/foo/bar")
        .build()
        .unwrap_or(request)
    };

    let request = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("request should build");

    assert_eq!(
      condition.rewrite(request).url().path(),
      "/foo/bar".to_string()
    );
  }
}

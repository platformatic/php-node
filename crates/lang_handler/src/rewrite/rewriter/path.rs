use regex::{Error, Regex};

use super::{Request, Rewriter};

/// Rewrite a request path using a given pattern and replacement.
pub struct PathRewriter {
  pattern: Regex,
  replacement: String,
}

impl PathRewriter {
  pub fn new<R, S>(pattern: R, replacement: S) -> Result<Self, Error>
  where
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
    S: Into<String>,
  {
    let pattern = pattern.try_into()?;
    let replacement = replacement.into();
    Ok(Self {
      pattern,
      replacement,
    })
  }
}

impl Rewriter for PathRewriter {
  fn rewrite(&self, request: Request) -> Request {
    let url = request.url();

    let PathRewriter {
      pattern,
      replacement,
    } = self;
    let path = pattern.replace(url.path(), replacement.clone());

    let mut copy = url.clone();
    copy.set_path(path.as_ref());

    request.extend().url(copy).build().unwrap_or(request)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_path_rewriter() {
    let rewriter = PathRewriter::new(r"^(/index.php)$", "/foo$1").expect("regex should be valid");

    let request = Request::builder()
      .url("http://example.com/index.php")
      .build()
      .expect("request should build");

    assert_eq!(
      rewriter.rewrite(request).url().path(),
      "/foo/index.php".to_string()
    );
  }
}

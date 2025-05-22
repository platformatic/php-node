use regex::{Error, Regex};

use super::{Request, Rewriter};

/// Rewrite a request header using a given pattern and replacement.
pub struct HeaderRewriter {
  name: String,
  pattern: Regex,
  replacement: String,
}

impl HeaderRewriter {
  pub fn new<N, R, S>(name: N, pattern: R, replacement: S) -> Result<Self, Error>
  where
    N: Into<String>,
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
    S: Into<String>,
  {
    let name = name.into();
    let pattern = pattern.try_into()?;
    let replacement = replacement.into();
    Ok(Self {
      name,
      pattern,
      replacement,
    })
  }
}

impl Rewriter for HeaderRewriter {
  fn rewrite(&self, request: Request) -> Request {
    let HeaderRewriter {
      name,
      pattern,
      replacement,
    } = self;

    match request.headers().get(name) {
      None => request,
      Some(value) => request
        .extend()
        .header(name, pattern.replace(&value, replacement.clone()))
        .build()
        .unwrap_or(request),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_header_rewriter() {
    let rewriter = HeaderRewriter::new("TEST", r"^foo$", "bar").expect("regex should be valid");

    let request = Request::builder()
      .url("http://example.com/index.php")
      .header("TEST", "foo")
      .build()
      .expect("request should build");

    assert_eq!(
      rewriter.rewrite(request).headers().get("TEST"),
      Some("bar".to_string())
    );
  }
}

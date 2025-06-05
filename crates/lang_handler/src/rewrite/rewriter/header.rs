use regex::{Error, Regex};

use super::{Request, Rewriter};

/// Rewrite a request header using a given pattern and replacement.
pub struct HeaderRewriter {
  name: String,
  pattern: Regex,
  replacement: String,
}

impl HeaderRewriter {
  /// Construct a new HeaderRewriter to replace the named header using the
  /// provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, HeaderRewriter};
  /// # use lang_handler::Request;
  /// let rewriter = HeaderRewriter::new("TEST", "(foo)", "$1bar")
  ///   .expect("should be valid regex");
  /// ```
  pub fn new<N, R, S>(name: N, pattern: R, replacement: S) -> Result<Box<Self>, Error>
  where
    N: Into<String>,
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
    S: Into<String>,
  {
    let name = name.into();
    let pattern = pattern.try_into()?;
    let replacement = replacement.into();
    Ok(Box::new(Self {
      name,
      pattern,
      replacement,
    }))
  }
}

impl Rewriter for HeaderRewriter {
  /// Rewrite named header using the provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, HeaderRewriter};
  /// # use lang_handler::Request;
  /// let rewriter = HeaderRewriter::new("TEST", "(foo)", "${1}bar")
  ///   .expect("should be valid regex");
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/index.php")
  ///   .header("TEST", "foo")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(
  ///   rewriter.rewrite(request).headers().get("TEST"),
  ///   Some("foobar".to_string())
  /// );
  /// ```
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

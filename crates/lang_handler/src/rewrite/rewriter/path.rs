use regex::{Error, Regex};

use super::{Request, Rewriter};

/// Rewrite a request path using a given pattern and replacement.
pub struct PathRewriter {
  pattern: Regex,
  replacement: String,
}

impl PathRewriter {
  /// Construct PathRewriter using the provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, PathRewriter};
  /// # use lang_handler::Request;
  /// let rewriter = PathRewriter::new("^(/foo)$", "/index.php")
  ///   .expect("should be valid regex");
  /// ```
  pub fn new<R, S>(pattern: R, replacement: S) -> Result<Box<Self>, Error>
  where
    R: TryInto<Regex>,
    Error: From<<R as TryInto<Regex>>::Error>,
    S: Into<String>,
  {
    let pattern = pattern.try_into()?;
    let replacement = replacement.into();
    Ok(Box::new(Self {
      pattern,
      replacement,
    }))
  }
}

impl Rewriter for PathRewriter {
  /// Rewrite request path using the provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, PathRewriter};
  /// # use lang_handler::Request;
  /// let rewriter = PathRewriter::new("^(/foo)$", "/index.php")
  ///   .expect("should be valid regex");
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/foo")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// assert_eq!(
  ///   rewriter.rewrite(request).url().path(),
  ///   "/index.php".to_string()
  /// );
  /// ```
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

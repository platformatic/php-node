use std::path::Path;

use regex::{Error, Regex};

use super::{Request, RequestBuilderException, Rewriter};

/// Rewrite a request header using a given pattern and replacement.
pub struct MethodRewriter(Regex, String);

impl MethodRewriter {
  /// Construct a new MethodRewriter to replace the Request method using the
  /// provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, MethodRewriter};
  /// # use lang_handler::Request;
  /// let rewriter = MethodRewriter::new("PUT", "POST")
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
    Ok(Box::new(Self(pattern, replacement)))
  }
}

impl Rewriter for MethodRewriter {
  /// Rewrite Request method using the provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, MethodRewriter};
  /// # use lang_handler::Request;
  /// # let docroot = std::env::temp_dir();
  /// let rewriter = MethodRewriter::new("PUT", "POST")
  ///   .expect("should be valid regex");
  ///
  /// let request = Request::builder()
  ///   .method("PUT")
  ///   .url("http://example.com/index.php")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let new_request = rewriter.rewrite(request, &docroot)
  ///   .expect("should rewrite request");
  ///
  /// assert_eq!(new_request.method(), "POST".to_string());
  /// ```
  fn rewrite(&self, request: Request, _docroot: &Path) -> Result<Request, RequestBuilderException> {
    let MethodRewriter(pattern, replacement) = self;

    let input = request.method();
    let output = pattern.replace(input, replacement.clone());
    if output == input {
      return Ok(request);
    }

    request.extend().method(output).build()
  }
}

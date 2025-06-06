use std::path::Path;

use regex::{Error, Regex};
use url::Url;

use super::{Request, RequestBuilderException, Rewriter};

/// Rewrite a request href using a given pattern and replacement.
pub struct HrefRewriter(Regex, String);

impl HrefRewriter {
  /// Construct HrefRewriter using the provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, HrefRewriter};
  /// # use lang_handler::Request;
  /// let rewriter = HrefRewriter::new("^(/foo)$", "/index.php")
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

impl Rewriter for HrefRewriter {
  /// Rewrite request path using the provided regex pattern and replacement.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::rewrite::{Rewriter, HrefRewriter};
  /// # use lang_handler::Request;
  /// # let docroot = std::env::temp_dir();
  /// let rewriter = HrefRewriter::new("^(.*)$", "/index.php?route=$1")
  ///   .expect("should be valid regex");
  ///
  /// let request = Request::builder()
  ///   .url("http://example.com/foo/bar")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// let new_request = rewriter.rewrite(request, &docroot)
  ///   .expect("should rewrite request");
  ///
  /// assert_eq!(new_request.url().path(), "/index.php".to_string());
  /// assert_eq!(new_request.url().query(), Some("route=/foo/bar"));
  /// ```
  fn rewrite(&self, request: Request, _docroot: &Path) -> Result<Request, RequestBuilderException> {
    let HrefRewriter(pattern, replacement) = self;
    let url = request.url();

    let input = {
      let path = url.path();
      let query = url.query().map_or(String::new(), |q| format!("?{}", q));
      let fragment = url.fragment().map_or(String::new(), |f| format!("#{}", f));
      format!("{}{}{}", path, query, fragment)
    };
    let output = pattern.replace(&input, replacement);

    // No change, return original request
    if input == output {
      return Ok(request);
    }

    let base_url_string = format!("{}://{}", url.scheme(), url.authority());
    let base_url = Url::parse(&base_url_string)
      .map_err(|_| RequestBuilderException::UrlParseFailed(base_url_string.clone()))?;

    let options = Url::options().base_url(Some(&base_url));

    let copy = options.parse(output.as_ref()).map_err(|_| {
      RequestBuilderException::UrlParseFailed(format!("{}{}", base_url_string, output))
    })?;

    request.extend().url(copy).build()
  }
}

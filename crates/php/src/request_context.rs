use ext_php_rs::zend::SapiGlobals;
use http_handler::{Request, ResponseBuilderExt, BodyBuffer, ResponseLog, HeaderMap, StatusCode, HeaderName, HeaderValue};
use bytes::BytesMut;
use std::{ffi::c_void, path::PathBuf};
use http_handler::request::Parts;

/// The request context for the PHP SAPI.
///
/// This has been redesigned to address all issues in FIXME.md:
/// - Uses request Parts to avoid RefUnwindSafe issues (#1)
/// - Stores mutable body for proper consumption (#2)
/// - Uses extension types to accumulate response data (#3, #4)
#[derive(Debug)]
pub struct RequestContext {
  request_parts: Parts,
  request_body: BytesMut,
  response_status: StatusCode,
  response_headers: HeaderMap,
  response_body: BodyBuffer,
  response_log: ResponseLog,
  response_exception: Option<String>,
  docroot: PathBuf,
}

impl RequestContext {
  /// Sets the current request context for the PHP SAPI.
  ///
  /// Uses into_parts() to avoid RefUnwindSafe issues (FIXME.md #1).
  pub fn for_request<S>(request: Request, docroot: S)
  where
    S: Into<PathBuf>,
  {
    // Use into_parts() to avoid RefUnwindSafe issues (FIXME.md #1)
    let (parts, body) = request.into_parts();

    let context = Box::new(RequestContext {
      request_parts: parts,
      request_body: body,
      response_status: StatusCode::OK,
      response_headers: HeaderMap::new(),
      response_body: BodyBuffer::new(),
      response_log: ResponseLog::new(),
      response_exception: None,
      docroot: docroot.into(),
    });
    let mut globals = SapiGlobals::get_mut();
    globals.server_context = Box::into_raw(context) as *mut c_void;
  }

  /// Retrieve a mutable reference to the request context
  pub fn current<'a>() -> Option<&'a mut RequestContext> {
    let ptr = {
      let globals = SapiGlobals::get();
      globals.server_context as *mut RequestContext
    };
    if ptr.is_null() {
      return None;
    }

    Some(unsafe { &mut *ptr })
  }

  /// Reclaim ownership of the RequestContext. Useful for dropping.
  #[allow(dead_code)]
  pub fn reclaim() -> Option<Box<RequestContext>> {
    let ptr = {
      let mut globals = SapiGlobals::get_mut();
      std::mem::replace(&mut globals.server_context, std::ptr::null_mut())
    };
    if ptr.is_null() {
      return None;
    }
    Some(unsafe { Box::from_raw(ptr as *mut RequestContext) })
  }

  /// Returns a reference to the request parts.
  /// This replaces the old request() method since we now use parts.
  pub fn request_parts(&self) -> &Parts {
    &self.request_parts
  }

  /// Returns a mutable reference to the request body.
  /// This allows proper consumption of the body (FIXME.md #2).
  pub fn request_body_mut(&mut self) -> &mut BytesMut {
    &mut self.request_body
  }

  /// Returns a reference to the request body.
  pub fn request_body(&self) -> &BytesMut {
    &self.request_body
  }

  /// Add a header to the response.
  pub fn add_response_header<K, V>(&mut self, key: K, value: V)
  where
    K: TryInto<HeaderName>,
    V: TryInto<HeaderValue>,
  {
    if let (Ok(header_name), Ok(header_value)) = (key.try_into(), value.try_into()) {
      self.response_headers.insert(header_name, header_value);
    }
  }

  /// Set the response status code.
  pub fn set_response_status(&mut self, status: u16) {
    if let Ok(status_code) = StatusCode::from_u16(status) {
      self.response_status = status_code;
    }
  }

  /// Write data to the response body.
  pub fn write_response_body(&mut self, data: &[u8]) {
    self.response_body.append(data);
  }

  /// Write to the response log.
  /// This uses extension types to accumulate log data (FIXME.md #4).
  pub fn write_response_log(&mut self, data: &[u8]) {
    self.response_log.append(data);
  }

  /// Set an exception on the response.
  /// This stores the exception to be added via ResponseBuilderExt (FIXME.md #3).
  pub fn set_response_exception(&mut self, exception: impl Into<String>) {
    self.response_exception = Some(exception.into());
  }

  /// Build the final response using the accumulated data.
  /// This properly uses ResponseBuilderExt for logs and exceptions (FIXME.md #3, #4).
  pub fn build_response(self) -> Result<http_handler::Response, http_handler::Error> {
    // Start building the response
    let mut builder = http_handler::response::Response::builder()
      .status(self.response_status);

    // Add all headers
    for (key, value) in &self.response_headers {
      builder = builder.header(key, value);
    }

    // Add extensions using ResponseBuilderExt
    builder = builder
      .body_buffer(self.response_body)
      .log(self.response_log.into_bytes());

    if let Some(exception) = self.response_exception {
      builder = builder.exception(exception);
    }

    // Get the body buffer from extensions and build final response
    let body = builder.extensions_mut()
      .and_then(|ext| ext.remove::<BodyBuffer>())
      .unwrap_or_default()
      .into_bytes_mut();

    builder.body(body)
  }

  /// Returns the docroot associated with this request context
  pub fn docroot(&self) -> PathBuf {
    self.docroot.to_owned()
  }
}

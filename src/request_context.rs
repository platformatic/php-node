use ext_php_rs::zend::SapiGlobals;
use http_handler::{BodyBuffer, Request};
use std::{ffi::c_void, path::PathBuf};

/// The request context for the PHP SAPI.
pub struct RequestContext {
  request: Request,
  response_builder: http_handler::response::Builder,
  docroot: PathBuf,
}

impl RequestContext {
  /// Sets the current request context for the PHP SAPI.
  pub fn for_request<S>(request: Request, docroot: S)
  where
    S: Into<PathBuf>,
  {
    let context = Box::new(RequestContext {
      request,
      response_builder: http_handler::response::Builder::new(),
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

  /// Returns a reference to the request.
  pub fn request(&self) -> &Request {
    &self.request
  }

  /// Returns a mutable reference to the request.
  pub fn request_mut(&mut self) -> &mut Request {
    &mut self.request
  }

  /// Returns a mutable reference to the response builder.
  pub fn response_builder_mut(&mut self) -> &mut http_handler::response::Builder {
    &mut self.response_builder
  }

  /// Build the final response using the accumulated data.
  pub fn build_response(mut self) -> Result<http_handler::Response, http_handler::Error> {
    // Extract the body buffer from extensions (if any was accumulated)
    let body = self
      .response_builder
      .extensions_mut()
      .and_then(|ext| ext.remove::<BodyBuffer>())
      .unwrap_or_default()
      .into_bytes_mut();

    self.response_builder.body(body)
  }

  /// Returns the docroot associated with this request context
  pub fn docroot(&self) -> PathBuf {
    self.docroot.to_owned()
  }
}

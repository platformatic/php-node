use ext_php_rs::zend::SapiGlobals;
use lang_handler::{Request, ResponseBuilder};
use std::ffi::c_void;

/// The request context for the PHP SAPI.
#[derive(Debug)]
pub struct RequestContext {
  request: Request,
  response_builder: ResponseBuilder,
}

impl RequestContext {
  /// Sets the current request context for the PHP SAPI.
  ///
  /// # Examples
  ///
  /// ```
  /// use php::{Request, RequestContext};
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request);
  ///
  /// let context = RequestContext::current()
  ///   .expect("should acquire current context");
  ///
  /// assert_eq!(context.request().method(), "GET");
  /// ```
  pub fn for_request(request: Request) {
    let context = Box::new(RequestContext {
      request,
      response_builder: ResponseBuilder::new(),
    });
    let mut globals = SapiGlobals::get_mut();
    globals.server_context = Box::into_raw(context) as *mut c_void;
  }

  /// Retrieve a mutable reference to the request context
  ///
  /// # Examples
  ///
  /// ```
  /// use php::{Request, RequestContext};
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request);
  ///
  /// let current_context = RequestContext::current()
  ///   .expect("should acquire current context");
  ///
  /// assert_eq!(current_context.request().method(), "GET");
  /// ```
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
  ///
  /// # Example
  ///
  /// ```
  /// use ext_php_rs::zend::SapiGlobals;
  /// use php::{Request, RequestContext};
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request);
  ///
  /// RequestContext::reclaim()
  ///   .expect("should acquire current context");
  ///
  /// assert_eq!(SapiGlobals::get().server_context, std::ptr::null_mut());
  /// ```
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
  ///
  /// # Examples
  ///
  /// ```
  /// use php::{Request, RequestContext};
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request);
  ///
  /// let context = RequestContext::current()
  ///   .expect("should acquire current context");
  ///
  /// assert_eq!(context.request().method(), "GET");
  /// ```
  pub fn request(&self) -> &Request {
    &self.request
  }

  /// Returns a mutable reference to the response builder.
  ///
  /// # Examples
  ///
  /// ```
  /// use php::{Request, RequestContext};
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request);
  ///
  /// let mut context = RequestContext::current()
  ///   .expect("should acquire current context");
  ///
  /// context.response_builder().status(200);
  /// ```
  pub fn response_builder(&mut self) -> &mut ResponseBuilder {
    &mut self.response_builder
  }
}

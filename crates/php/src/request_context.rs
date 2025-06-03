use ext_php_rs::zend::SapiGlobals;
use lang_handler::{Request, ResponseBuilder};
use std::{ffi::c_void, path::PathBuf};

/// The request context for the PHP SAPI.
#[derive(Debug)]
pub struct RequestContext {
  request: Request,
  response_builder: ResponseBuilder,
  docroot: PathBuf,
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
  ///   .url("http://example.com").expect("should parse url")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request, "/foo");
  ///
  /// let context = RequestContext::current()
  ///   .expect("should acquire current context");
  ///
  /// assert_eq!(context.request().method(), "GET");
  /// ```
  pub fn for_request<S>(request: Request, docroot: S)
  where
    S: Into<PathBuf>,
  {
    let context = Box::new(RequestContext {
      request,
      response_builder: ResponseBuilder::new(),
      docroot: docroot.into(),
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
  ///   .url("http://example.com").expect("should parse url")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request, "/foo");
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
  ///   .url("http://example.com").expect("should parse url")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request, "/foo");
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
  ///   .url("http://example.com").expect("should parse url")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request, "/foo");
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
  ///   .url("http://example.com").expect("should parse url")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request, "/foo");
  ///
  /// let mut context = RequestContext::current()
  ///   .expect("should acquire current context");
  ///
  /// context.response_builder().status(200);
  /// ```
  pub fn response_builder(&mut self) -> &mut ResponseBuilder {
    &mut self.response_builder
  }

  /// Returns the docroot associated with this request context
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::path::PathBuf;
  /// use php::{Request, RequestContext};
  ///
  /// let request = Request::builder()
  ///   .method("GET")
  ///   .url("http://example.com").expect("should parse url")
  ///   .build()
  ///   .expect("should build request");
  ///
  /// RequestContext::for_request(request, "/foo");
  ///
  /// let mut context = RequestContext::current()
  ///   .expect("should acquire current context");
  ///
  /// assert_eq!(context.docroot(), PathBuf::new().join("/foo"));
  /// ```
  pub fn docroot(&self) -> PathBuf {
    self.docroot.to_owned()
  }
}

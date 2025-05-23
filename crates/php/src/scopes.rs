use ext_php_rs::ffi::{php_request_shutdown, php_request_startup, ZEND_RESULT_CODE_SUCCESS};
use std::ffi::c_void;

use crate::EmbedException;

/// A scope in which php request activity may occur. This is responsible for
/// starting up and shutting down the php request and cleaning up associated
/// data.
pub(crate) struct RequestScope();

impl RequestScope {
  /// Starts a new request scope in which a PHP request may operate.
  pub fn new() -> Result<Self, EmbedException> {
    if unsafe { php_request_startup() } != ZEND_RESULT_CODE_SUCCESS {
      return Err(EmbedException::SapiRequestNotStarted);
    }

    Ok(RequestScope())
  }
}

impl Drop for RequestScope {
  fn drop(&mut self) {
    unsafe {
      php_request_shutdown(std::ptr::null_mut::<c_void>());
    }
  }
}

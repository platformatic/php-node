use std::{
  ffi::{c_char, c_void},
  ops::{Deref, DerefMut},
  path::Path,
};

use ext_php_rs::{
  alloc::{efree, estrdup},
  ffi::{
    _zend_file_handle__bindgen_ty_1, php_request_shutdown, php_request_startup,
    zend_destroy_file_handle, zend_file_handle, zend_stream_init_filename,
    ZEND_RESULT_CODE_SUCCESS,
  },
};

use super::EmbedRequestError;

/// A scope in which php request activity may occur. This is responsible for
/// starting up and shutting down the php request and cleaning up associated
/// data.
pub(crate) struct RequestScope();

impl RequestScope {
  /// Starts a new request scope in which a PHP request may operate.
  pub fn new() -> Result<Self, EmbedRequestError> {
    if unsafe { php_request_startup() } != ZEND_RESULT_CODE_SUCCESS {
      return Err(EmbedRequestError::SapiRequestNotStarted);
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

pub(crate) struct FileHandleScope(zend_file_handle, *mut c_char);

impl FileHandleScope {
  pub fn new<P>(path: P) -> Self
  where
    P: AsRef<Path>,
  {
    let mut handle = zend_file_handle {
      handle: _zend_file_handle__bindgen_ty_1 {
        fp: std::ptr::null_mut(),
      },
      filename: std::ptr::null_mut(),
      opened_path: std::ptr::null_mut(),
      type_: 0, //ZEND_HANDLE_FP
      primary_script: false,
      in_list: false,
      buf: std::ptr::null_mut(),
      len: 0,
    };

    let path = estrdup(path.as_ref().to_str().unwrap());

    unsafe {
      zend_stream_init_filename(&mut handle, path);
    }
    handle.primary_script = true;

    Self(handle, path)
  }
}

impl Deref for FileHandleScope {
  type Target = zend_file_handle;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for FileHandleScope {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl Drop for FileHandleScope {
  fn drop(&mut self) {
    unsafe {
      zend_destroy_file_handle(&mut self.0);
      efree(self.1 as *mut u8);
    };
  }
}

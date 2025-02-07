use std::{env::Args, ffi::{CStr, CString}, ops::Deref};

use crate::sys;

#[derive(Debug, Clone)]
pub struct Response {
    response: *mut sys::php_http_response
}

impl Deref for Response {
    type Target = *mut sys::php_http_response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl From<*mut sys::php_http_response> for Response {
    fn from(response: *mut sys::php_http_response) -> Self {
        Response {
            response
        }
    }
}

impl Response {
    pub fn status(&self) -> i32 {
        unsafe {
            sys::php_http_response_get_status(self.response)
        }
    }

    pub fn set_status(&self, status: i32) {
        unsafe {
            sys::php_http_response_set_status(self.response, status);
        }
    }

    pub fn body(&self) -> String {
        unsafe {
            let body = sys::php_http_response_get_body(self.response);
            CStr::from_ptr(body).to_string_lossy().into_owned()
        }
    }

    pub fn set_body<T>(&self, body: T)
    where
        T: AsRef<str>
    {
        unsafe {
            let str: CString = CString::new(body.as_ref()).unwrap();
            sys::php_http_response_set_body(self.response, str.as_ptr());
        }
    }
}

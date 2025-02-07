use std::{env::Args, ffi::{CStr, CString}, ops::Deref};

use crate::sys;

#[derive(Debug, Clone)]
pub struct Request {
    request: *mut sys::php_http_request
}

impl Deref for Request {
    type Target = *mut sys::php_http_request;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl From<*mut sys::php_http_request> for Request {
    fn from(request: *mut sys::php_http_request) -> Self {
        Request { request }
    }
}

impl Request {
    pub fn new() -> Self {
        unsafe { sys::php_http_request_new() }.into()
    }

    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    // Method
    pub fn set_method<T>(&self, method: T)
    where
        T: AsRef<str>
    {
        unsafe {
            let str: CString = CString::new(method.as_ref()).unwrap();
            sys::php_http_request_set_method(self.request, str.as_ptr());
        }
    }
    pub fn method(&self) -> String {
        unsafe {
            let method = sys::php_http_request_get_method(self.request);
            CStr::from_ptr(method).to_string_lossy().into_owned()
        }
    }

    // URI
    pub fn set_path<T>(&self, path: T)
    where
        T: AsRef<str>
    {
        unsafe {
            let str: CString = CString::new(path.as_ref()).unwrap();
            sys::php_http_request_set_path(self.request, str.as_ptr());
        }
    }
    pub fn path(&self) -> String {
        unsafe {
            let path = sys::php_http_request_get_path(self.request);
            CStr::from_ptr(path).to_string_lossy().into_owned()
        }
    }

    // Body
    // TODO: Streaming bodies with futures::Stream
    pub fn set_body<T>(&self, body: T)
    where
        T: AsRef<str>
    {
        unsafe {
            let str: CString = CString::new(body.as_ref()).unwrap();
            sys::php_http_request_set_body(self.request, str.as_ptr());
        }
    }
    pub fn body(&self) -> String {
        unsafe {
            let body = sys::php_http_request_get_body(self.request);
            CStr::from_ptr(body).to_string_lossy().into_owned()
        }
    }
}

pub struct RequestBuilder {
    request: Request
}

impl RequestBuilder {
    pub fn new() -> Self {
        RequestBuilder {
            request: Request::new()
        }
    }

    pub fn method<T>(self, method: T) -> Self
    where
        T: AsRef<str>
    {
        self.request.set_method(method);
        self
    }

    pub fn path<T>(self, path: T) -> Self
    where
        T: AsRef<str>
    {
        self.request.set_path(path);
        self
    }

    pub fn body<T>(self, body: T) -> Self
    where
        T: AsRef<str>
    {
        self.request.set_body(body);
        self
    }

    pub fn build(self) -> Request {
        self.request
    }
}

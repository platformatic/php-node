#[cfg(feature = "c")]
use std::ffi::{CStr, CString, c_char};

use crate::Headers;
use crate::headers::lh_headers_t;

#[derive(Clone)]
pub struct Response {
    status: u16,
    headers: Headers,
    // TODO: Support Stream bodies when napi.rs supports it
    body: String,
}

impl Response {
    pub fn new(status: u16, headers: Headers, body: String) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn extend(&self) -> ResponseBuilder {
        ResponseBuilder::extend(self)
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn body(&self) -> &String {
        &self.body
    }
}

#[derive(Clone)]
pub struct ResponseBuilder {
    status: Option<u16>,
    headers: Headers,
    body: Option<String>,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            status: None,
            headers: Headers::new(),
            body: None,
        }
    }

    pub fn extend(response: &Response) -> Self {
        ResponseBuilder {
            status: Some(response.status),
            headers: response.headers.clone(),
            body: Some(response.body.clone()),
        }
    }

    pub fn status_code(&mut self, status: u16) -> &mut Self {
        self.status = Some(status);
        self
    }

    pub fn header<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.headers.set(key, value);
        self
    }

    pub fn body<B>(&mut self, body: B) -> &mut Self
    where
        B: Into<String>,
    {
        self.body = Some(body.into());
        self
    }

    pub fn build(&self) -> Response {
        Response {
            status: self.status.unwrap_or(200),
            headers: self.headers.clone(),
            body: self.body.clone().unwrap_or_default(),
        }
    }
}

#[allow(non_camel_case_types)]
pub struct lh_response_t {
    inner: Response,
}

impl From<Response> for lh_response_t {
    fn from(inner: Response) -> Self {
        Self { inner }
    }
}

impl From<&lh_response_t> for Response {
    fn from(response: &lh_response_t) -> Response {
        response.inner.clone()
    }
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_new(status_code: u16, headers: *mut lh_headers_t, body: *const c_char) -> *mut lh_response_t {
    let body_str = unsafe { CStr::from_ptr(body).to_string_lossy().into_owned() };
    let headers = unsafe { &*headers };
    Box::into_raw(Box::new(Response::new(status_code, headers.into(), body_str).into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_free(response: *mut lh_response_t) {
    if !response.is_null() {
        unsafe {
            drop(Box::from_raw(response));
        }
    }
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_status(response: *const lh_response_t) -> u16 {
    let response = unsafe { &*response };
    response.inner.status()
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_headers(response: *const lh_response_t) -> *mut lh_headers_t {
    let response = unsafe { &*response };
    Box::into_raw(Box::new(response.inner.headers().clone().into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_body(response: *const lh_response_t) -> *const c_char {
    let response = unsafe { &*response };
    CString::new(response.inner.body().as_str().as_bytes()).unwrap().into_raw()
}

#[allow(non_camel_case_types)]
pub struct lh_response_builder_t {
    inner: ResponseBuilder,
}

impl From<ResponseBuilder> for lh_response_builder_t {
    fn from(inner: ResponseBuilder) -> Self {
        Self { inner }
    }
}

impl From<&lh_response_builder_t> for ResponseBuilder {
    fn from(builder: &lh_response_builder_t) -> ResponseBuilder {
        builder.inner.clone()
    }
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_builder_new() -> *mut lh_response_builder_t {
    Box::into_raw(Box::new(ResponseBuilder::new().into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_builder_extend(response: *const lh_response_t) -> *mut lh_response_builder_t {
    let response = unsafe { &*response };
    Box::into_raw(Box::new(ResponseBuilder::extend(&response.inner).into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_builder_status_code(builder: *mut lh_response_builder_t, status_code: u16) {
    let builder = unsafe { &mut *builder };
    builder.inner.status_code(status_code);
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_builder_header(builder: *mut lh_response_builder_t, key: *const c_char, value: *const c_char) {
    let builder = unsafe { &mut *builder };
    let key_str = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    let value_str = unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() };
    builder.inner.header(key_str, value_str);
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_builder_body(builder: *mut lh_response_builder_t, body: *const c_char) {
    let builder = unsafe { &mut *builder };
    let body_str = unsafe { CStr::from_ptr(body).to_string_lossy().into_owned() };
    builder.inner.body(body_str);
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_response_builder_build(builder: *const lh_response_builder_t) -> *mut lh_response_t {
    let builder = unsafe { &*builder };
    Box::into_raw(Box::new(builder.inner.build().into()))
}

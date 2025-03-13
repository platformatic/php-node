#[cfg(feature = "c")]
use std::{ffi, ffi::{CStr, CString}};

use std::fmt::Debug;

use bytes::{Buf, Bytes, BytesMut};
use url::{ParseError, Url};

use crate::Headers;
use crate::headers::lh_headers_t;
use crate::url::lh_url_t;

#[derive(Clone, Debug)]
pub struct Request {
    method: String,
    url: Url,
    headers: Headers,
    // TODO: Support Stream bodies when napi.rs supports it
    body: Bytes,
}

impl Request {
    pub fn new<T: Into<Bytes>>(method: String, url: Url, headers: Headers, body: T) -> Self {
        Self {
            method,
            url,
            headers,
            body: body.into()
        }
    }

    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    pub fn extend(&self) -> RequestBuilder {
        RequestBuilder::extend(self)
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn body(&self) -> Bytes {
        self.body.clone()
    }
}

#[derive(Clone)]
pub struct RequestBuilder {
    method: Option<String>,
    url: Option<Url>,
    headers: Headers,
    body: BytesMut,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            method: None,
            url: None,
            headers: Headers::new(),
            body: BytesMut::with_capacity(1024),
        }
    }

    pub fn extend(request: &Request) -> Self {
        Self {
            method: Some(request.method().into()),
            url: Some(request.url().clone()),
            headers: request.headers().clone(),
            body: BytesMut::from(request.body()),
        }
    }

    pub fn method<T: Into<String>>(mut self, method: T) -> Self {
        self.method = Some(method.into());
        self
    }

    pub fn url<T>(mut self, url: T) -> Result<Self, ParseError>
    where
        T: Into<String>
    {
        match url.into().parse() {
            Ok(url) => {
                self.url = Some(url);
                Ok(self)
            },
            Err(e) => return Err(e),
        }
    }

    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>
    {
        self.headers.set(key.into(), value.into());
        self
    }

    pub fn body<T: Into<BytesMut>>(mut self, body: T) -> Self {
        self.body = body.into();
        self
    }

    pub fn build(self) -> Request {
        Request {
            method: self.method.unwrap_or_else(|| "GET".to_string()),
            url: self.url.unwrap_or_else(|| Url::parse("/").unwrap()),
            headers: self.headers,
            body: self.body.freeze(),
        }
    }
}

#[allow(non_camel_case_types)]
pub struct lh_request_t {
    inner: Request,
}

impl From<Request> for lh_request_t {
    fn from(inner: Request) -> Self {
        Self { inner }
    }
}

impl From<&lh_request_t> for Request {
    fn from(request: &lh_request_t) -> Request {
        request.inner.clone()
    }
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_new(
    method: *const ffi::c_char,
    url: *const ffi::c_char,
    headers: *mut lh_headers_t,
    body: *const ffi::c_char,
) -> *mut lh_request_t {
    let method = unsafe { CStr::from_ptr(method).to_string_lossy().into_owned() };
    let url_str = unsafe { CStr::from_ptr(url).to_string_lossy().into_owned() };
    let url = Url::parse(&url_str).unwrap();
    let body = if body.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(body).to_bytes() })
    };
    let headers = unsafe { &*headers };
    let request = Request::new(method, url, headers.into(), body.unwrap_or(&[]));
    Box::into_raw(Box::new(request.into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_method(request: *const lh_request_t) -> *const ffi::c_char {
    let request = unsafe { &*request };
    CString::new(request.inner.method()).unwrap().into_raw()
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_url(request: *const lh_request_t) -> *mut lh_url_t {
    let request = unsafe { &*request };
    Box::into_raw(Box::new(request.inner.url().clone().into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_headers(request: *const lh_request_t) -> *mut lh_headers_t {
    let request = unsafe { &*request };
    Box::into_raw(Box::new(request.inner.headers().clone().into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_body(request: *const lh_request_t) -> *const ffi::c_char {
    let request = unsafe { &*request };
    CString::new(request.inner.body()).unwrap().into_raw()
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_body_read(request: *const lh_request_t, buffer: *mut ffi::c_char, length: usize) -> usize {
    let request = unsafe { &*request };
    let body = request.inner.body();

    let length = length.min(body.len());
    let chunk = body.take(length);

    unsafe {
        std::ptr::copy_nonoverlapping(chunk.chunk().as_ptr() as *mut ffi::c_char, buffer, length);
    }
    length
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_free(request: *mut lh_request_t) {
    if !request.is_null() {
        unsafe {
            drop(Box::from_raw(request));
        }
    }
}

#[allow(non_camel_case_types)]
pub struct lh_request_builder_t {
    inner: RequestBuilder,
}

impl From<RequestBuilder> for lh_request_builder_t {
    fn from(inner: RequestBuilder) -> Self {
        Self { inner }
    }
}

impl From<&lh_request_builder_t> for RequestBuilder {
    fn from(builder: &lh_request_builder_t) -> RequestBuilder {
        builder.inner.clone()
    }
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_new() -> *mut lh_request_builder_t {
    Box::into_raw(Box::new(RequestBuilder::new().into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_extend(request: *const lh_request_t) -> *mut lh_request_builder_t {
    let request = unsafe { &*request };
    Box::into_raw(Box::new(RequestBuilder::extend(&request.inner).into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_method(
    builder: *mut lh_request_builder_t,
    method: *const ffi::c_char,
) -> *mut lh_request_builder_t {
    let method = unsafe { CStr::from_ptr(method).to_string_lossy().into_owned() };
    let builder = unsafe { &mut *builder };
    Box::into_raw(Box::new(builder.inner.clone().method(method).into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_url(
    builder: *mut lh_request_builder_t,
    url: *const ffi::c_char,
) -> *mut lh_request_builder_t {
    let url = unsafe { CStr::from_ptr(url).to_string_lossy().into_owned() };
    let builder = unsafe { &mut *builder };
    Box::into_raw(Box::new(builder.inner.clone().url(&url).unwrap().into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_header(
    builder: *mut lh_request_builder_t,
    key: *const ffi::c_char,
    value: *const ffi::c_char,
) -> *mut lh_request_builder_t {
    let key = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    let value = unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() };
    let builder = unsafe { &mut *builder };
    Box::into_raw(Box::new(builder.inner.clone().header(key, value).into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_body(
    builder: *mut lh_request_builder_t,
    body: *const ffi::c_char,
) -> *mut lh_request_builder_t {
    let body = unsafe { CStr::from_ptr(body).to_bytes() };
    let builder = unsafe { &mut *builder };
    Box::into_raw(Box::new(builder.inner.clone().body(body).into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_build(builder: *mut lh_request_builder_t) -> *mut lh_request_t {
    let builder = unsafe { Box::from_raw(builder) };
    Box::into_raw(Box::new(builder.inner.build().into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_request_builder_free(builder: *mut lh_request_builder_t) {
    if !builder.is_null() {
        unsafe {
            drop(Box::from_raw(builder));
        }
    }
}

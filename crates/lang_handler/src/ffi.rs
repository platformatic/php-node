use std::{ffi, ffi::{CStr, CString, c_char}};

use bytes::{Buf, BufMut};

use crate::{Headers, Request, RequestBuilder, Response, ResponseBuilder, Url};

/// Reclaim a string allocated by the library.
///
/// # Examples
///
/// ```c
/// const char* rust_string = get_string_from_rust();
/// // Return ownership of the string to Rust and drop
/// lh_reclaim_str(rust_string);
/// ```
#[no_mangle]
pub extern "C" fn lh_reclaim_str(url: *const c_char) {
    unsafe {
        drop(CString::from_raw(url as *mut c_char));
    }
}

/// A multi-map of HTTP headers. Each key can have multiple values.
#[allow(non_camel_case_types)]
pub struct lh_headers_t {
    inner: Headers,
}

/// Convert a `Headers` into a `lh_headers_t`.
impl From<Headers> for lh_headers_t {
    fn from(inner: Headers) -> Self {
        Self { inner }
    }
}

/// Convert a `&lh_headers_t` into a `Headers`.
impl From<&lh_headers_t> for Headers {
    fn from(headers: &lh_headers_t) -> Headers {
        headers.inner.clone()
    }
}

/// Create a new `lh_headers_t`.
///
/// # Examples
///
/// ```c
/// lh_headers_t* headers = lh_headers_new();
/// ```
#[no_mangle]
pub extern "C" fn lh_headers_new() -> *mut lh_headers_t {
    let headers = Headers::new();
    Box::into_raw(Box::new(headers.into()))
}

/// Free a `lh_headers_t`.
///
/// # Examples
///
/// ```c
/// lh_headers_t* headers = lh_headers_new();
///
/// // Do something...
///
/// lh_headers_free(headers);
/// ```
#[no_mangle]
pub extern "C" fn lh_headers_free(headers: *mut lh_headers_t) {
    if !headers.is_null() {
        unsafe {
            drop(Box::from_raw(headers));
        }
    }
}

/// Get the number of headers with the given key.
///
/// # Examples
///
/// ```c
/// lh_headers_t* headers = lh_headers_new();
/// size_t count = lh_headers_count(headers, "Accept");
/// ```
#[no_mangle]
pub extern "C" fn lh_headers_count(headers: *const lh_headers_t, key: *const std::os::raw::c_char) -> usize {
    let headers = unsafe {
        assert!(!headers.is_null());
        &*headers
    };
    let key = unsafe {
        assert!(!key.is_null());
        std::ffi::CStr::from_ptr(key).to_str().unwrap()
    };
    match headers.inner.get(key) {
        Some(value) => value.len(),
        None => 0
    }
}

/// Get the value of the last header with the given key.
///
/// # Examples
///
/// ```c
/// lh_headers_t* headers = lh_headers_new();
/// const char* value = lh_headers_get(headers, "Accept");
/// ```
#[no_mangle]
pub extern "C" fn lh_headers_get(headers: *const lh_headers_t, key: *const std::os::raw::c_char) -> *const std::os::raw::c_char {
    let headers = unsafe {
        assert!(!headers.is_null());
        &*headers
    };
    let key = unsafe {
        assert!(!key.is_null());
        std::ffi::CStr::from_ptr(key).to_str().unwrap()
    };
    match headers.inner.get(key) {
        Some(values) => {
            if values.len() > 0 {
                values[values.len() - 1].as_ptr() as *const std::os::raw::c_char
            } else {
                std::ptr::null()
            }
        },
        None => std::ptr::null()
    }
}

/// Get the value of the nth header with the given key.
///
/// # Examples
///
/// ```c
/// lh_headers_t* headers = lh_headers_new();
/// const char* value = lh_headers_get_nth(headers, "Accept", 0);
/// ```
#[no_mangle]
pub extern "C" fn lh_headers_get_nth(headers: *const lh_headers_t, key: *const std::os::raw::c_char, index: usize) -> *const std::os::raw::c_char {
    let headers = unsafe {
        assert!(!headers.is_null());
        &*headers
    };
    let key = unsafe {
        assert!(!key.is_null());
        std::ffi::CStr::from_ptr(key).to_str().unwrap()
    };
    match headers.inner.get(key) {
        Some(values) => {
            if index < values.len() {
                values[index].as_ptr() as *const std::os::raw::c_char
            } else {
                std::ptr::null()
            }
        },
        None => std::ptr::null()
    }
}

/// Set a header with the given key and value.
///
/// # Examples
///
/// ```c
/// lh_headers_t* headers = lh_headers_new();
/// lh_headers_set(headers, "Accept", "application/json");
/// ```
#[no_mangle]
pub extern "C" fn lh_headers_set(headers: *mut lh_headers_t, key: *const std::os::raw::c_char, value: *const std::os::raw::c_char) {
    let headers = unsafe {
        assert!(!headers.is_null());
        &mut *headers
    };
    let key = unsafe {
        assert!(!key.is_null());
        std::ffi::CStr::from_ptr(key).to_str().unwrap().to_string()
    };
    let value = unsafe {
        assert!(!value.is_null());
        std::ffi::CStr::from_ptr(value).to_str().unwrap().to_string()
    };
    headers.inner.set(key, value);
}

/// An HTTP request. Includes method, URL, headers, and body.
#[allow(non_camel_case_types)]
pub struct lh_request_t {
    inner: Request,
}

/// Convert a `Request` into a `lh_request_t`.
impl From<Request> for lh_request_t {
    fn from(inner: Request) -> Self {
        Self { inner }
    }
}

/// Convert a `&lh_request_t` into a `Request`.
impl From<&lh_request_t> for Request {
    fn from(request: &lh_request_t) -> Request {
        request.inner.clone()
    }
}

/// Create a new `lh_request_t`.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
/// ```
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

/// Free a `lh_request_t`.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
///
/// // Do something...
///
/// lh_request_free(request);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_free(request: *mut lh_request_t) {
    if !request.is_null() {
        unsafe {
            drop(Box::from_raw(request));
        }
    }
}

/// Get the method of the request.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
/// const char* method = lh_request_method(request);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_method(request: *const lh_request_t) -> *const ffi::c_char {
    let request = unsafe { &*request };
    CString::new(request.inner.method()).unwrap().into_raw()
}

/// Get the URL of the request.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
/// lh_url_t* url = lh_request_url(request);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_url(request: *const lh_request_t) -> *mut lh_url_t {
    let request = unsafe { &*request };
    Box::into_raw(Box::new(request.inner.url().clone().into()))
}

/// Get the headers of the request.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
/// lh_headers_t* headers = lh_request_headers(request);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_headers(request: *const lh_request_t) -> *mut lh_headers_t {
    let request = unsafe { &*request };
    Box::into_raw(Box::new(request.inner.headers().clone().into()))
}

/// Get the body of the request.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
/// const char* body = lh_request_body(request);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_body(request: *const lh_request_t) -> *const ffi::c_char {
    let request = unsafe { &*request };
    CString::new(request.inner.body()).unwrap().into_raw()
}

/// Read from the body of the request into a buffer. Consumes that many bytes from the body.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
/// char buffer[1024];
/// size_t length = lh_request_body_read(request, buffer, 1024);
/// ```
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

/// An HTTP request builder. Includes method, URL, headers, and body.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
/// ```
#[allow(non_camel_case_types)]
pub struct lh_request_builder_t {
    inner: RequestBuilder,
}

/// Convert a `RequestBuilder` into a `lh_request_builder_t`.
impl From<RequestBuilder> for lh_request_builder_t {
    fn from(inner: RequestBuilder) -> Self {
        Self { inner }
    }
}

/// Convert a `&lh_request_builder_t` into a `RequestBuilder`.
impl From<&lh_request_builder_t> for RequestBuilder {
    fn from(builder: &lh_request_builder_t) -> RequestBuilder {
        builder.inner.clone()
    }
}

/// Create a new `lh_request_builder_t`.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
/// ```
#[no_mangle]
pub extern "C" fn lh_request_builder_new() -> *mut lh_request_builder_t {
    Box::into_raw(Box::new(RequestBuilder::new().into()))
}

/// Free a `lh_request_builder_t`.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
///
/// // Do something...
///
/// lh_request_builder_free(builder);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_builder_free(builder: *mut lh_request_builder_t) {
    if !builder.is_null() {
        unsafe {
            drop(Box::from_raw(builder));
        }
    }
}

/// Create a new `lh_request_builder_t` from an existing `lh_request_t`.
///
/// # Examples
///
/// ```c
/// lh_request_t* request = lh_request_new("GET", "https://example.com", headers, "Hello, world!");
/// lh_request_builder_t* builder = lh_request_builder_extend(request);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_builder_extend(request: *const lh_request_t) -> *mut lh_request_builder_t {
    let request = unsafe { &*request };
    Box::into_raw(Box::new(RequestBuilder::extend(&request.inner).into()))
}

/// Set the method of the request.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
/// lh_request_builder_method(builder, "GET");
/// ```
#[no_mangle]
pub extern "C" fn lh_request_builder_method(
    builder: *mut lh_request_builder_t,
    method: *const ffi::c_char,
) -> *mut lh_request_builder_t {
    let method = unsafe { CStr::from_ptr(method).to_string_lossy().into_owned() };
    let builder = unsafe { &mut *builder };
    Box::into_raw(Box::new(builder.inner.clone().method(method).into()))
}

/// Set the URL of the request.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
/// lh_request_builder_url(builder, "https://example.com");
/// ```
#[no_mangle]
pub extern "C" fn lh_request_builder_url(
    builder: *mut lh_request_builder_t,
    url: *const ffi::c_char,
) -> *mut lh_request_builder_t {
    let url = unsafe { CStr::from_ptr(url).to_string_lossy().into_owned() };
    let builder = unsafe { &mut *builder };
    Box::into_raw(Box::new(builder.inner.clone().url(&url).unwrap().into()))
}

/// Add a header to the request.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
/// lh_request_builder_header(builder, "Content-Type", "text/plain");
/// ```
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

/// Set the body of the request.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
/// lh_request_builder_body(builder, "Hello, world!");
/// ```
#[no_mangle]
pub extern "C" fn lh_request_builder_body(
    builder: *mut lh_request_builder_t,
    body: *const ffi::c_char,
) -> *mut lh_request_builder_t {
    let body = unsafe { CStr::from_ptr(body).to_bytes() };
    let builder = unsafe { &mut *builder };
    Box::into_raw(Box::new(builder.inner.clone().body(body).into()))
}

/// Build a `lh_request_t` from a `lh_request_builder_t`.
///
/// # Examples
///
/// ```c
/// lh_request_builder_t* builder = lh_request_builder_new();
///
/// // Populate the builder data...
///
/// lh_request_t* request = lh_request_builder_build(builder);
/// ```
#[no_mangle]
pub extern "C" fn lh_request_builder_build(builder: *mut lh_request_builder_t) -> *mut lh_request_t {
    let builder = unsafe { Box::from_raw(builder) };
    Box::into_raw(Box::new(builder.inner.build().into()))
}

/// An HTTP response. Includes status code, headers, and body.
#[allow(non_camel_case_types)]
pub struct lh_response_t {
    inner: Response,
}

/// Convert a `Response` into a `lh_response_t`.
impl From<Response> for lh_response_t {
    fn from(inner: Response) -> Self {
        Self { inner }
    }
}

/// Convert a `&lh_response_t` into a `Response`.
impl From<&lh_response_t> for Response {
    fn from(response: &lh_response_t) -> Response {
        response.inner.clone()
    }
}

/// Create a new `lh_response_t`.
///
/// # Examples
///
/// ```c
/// lh_response_t* response = lh_response_new(200, headers, "Hello, world!");
/// ```
#[no_mangle]
pub extern "C" fn lh_response_new(status_code: u16, headers: *mut lh_headers_t, body: *const c_char) -> *mut lh_response_t {
    let body_str = unsafe { CStr::from_ptr(body).to_bytes() };
    let headers = unsafe { &*headers };
    Box::into_raw(Box::new(Response::new(status_code, headers.into(), body_str, "", None).into()))
}

/// Free a `lh_response_t`.
///
/// # Examples
///
/// ```c
/// lh_response_t* response = lh_response_new(200, headers, "Hello, world!");
///
/// // Do something...
///
/// lh_response_free(response);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_free(response: *mut lh_response_t) {
    if !response.is_null() {
        unsafe {
            drop(Box::from_raw(response));
        }
    }
}

/// Get the status code of the response.
///
/// # Examples
///
/// ```c
/// lh_response_t* response = lh_response_new(200, headers, "Hello, world!");
/// uint16_t status = lh_response_status(response);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_status(response: *const lh_response_t) -> u16 {
    let response = unsafe { &*response };
    response.inner.status()
}

/// Get the headers of the response.
///
/// # Examples
///
/// ```c
/// lh_response_t* response = lh_response_new(200, headers, "Hello, world!");
/// lh_headers_t* headers = lh_response_headers(response);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_headers(response: *const lh_response_t) -> *mut lh_headers_t {
    let response = unsafe { &*response };
    Box::into_raw(Box::new(response.inner.headers().clone().into()))
}

/// Get the body of the response.
///
/// # Examples
///
/// ```c
/// lh_response_t* response = lh_response_new(200, headers, "Hello, world!");
/// const char* body = lh_response_body(response);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_body(response: *const lh_response_t) -> *const c_char {
    let response = unsafe { &*response };
    CString::new(response.inner.body()).unwrap().into_raw()
}

/// An HTTP response builder. Includes status, headers, body, log, and exception string.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// ```
#[allow(non_camel_case_types)]
pub struct lh_response_builder_t {
    inner: ResponseBuilder,
}

/// Convert a `ResponseBuilder` into a `lh_response_builder_t`.
impl From<ResponseBuilder> for lh_response_builder_t {
    fn from(inner: ResponseBuilder) -> Self {
        Self { inner }
    }
}

/// Convert a `&lh_response_builder_t` into a `ResponseBuilder`.
impl From<&lh_response_builder_t> for ResponseBuilder {
    fn from(builder: &lh_response_builder_t) -> ResponseBuilder {
        builder.inner.clone()
    }
}

/// Create a new `lh_response_builder_t`.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_new() -> *mut lh_response_builder_t {
    Box::into_raw(Box::new(ResponseBuilder::new().into()))
}

/// Free a `lh_response_builder_t`.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
///
/// // Do something...
///
/// lh_response_builder_free(builder);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_free(builder: *mut lh_response_builder_t) {
    if !builder.is_null() {
        unsafe {
            drop(Box::from_raw(builder));
        }
    }
}

/// Create a new `lh_response_builder_t` from an existing `lh_response_t`.
///
/// # Examples
///
/// ```c
/// lh_response_t* response = lh_response_new(200, headers, "Hello, world!");
/// lh_response_builder_t* builder = lh_response_builder_extend(response);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_extend(response: *const lh_response_t) -> *mut lh_response_builder_t {
    let response = unsafe { &*response };
    Box::into_raw(Box::new(ResponseBuilder::extend(&response.inner).into()))
}

/// Set the status code of the response.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// lh_response_builder_status_code(builder, 200);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_status_code(builder: *mut lh_response_builder_t, status_code: u16) {
    let builder = unsafe { &mut *builder };
    builder.inner.status(status_code);
}

/// Add a header to the response.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// lh_response_builder_header(builder, "Content-Type", "text/plain");
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_header(builder: *mut lh_response_builder_t, key: *const c_char, value: *const c_char) {
    let builder = unsafe { &mut *builder };
    let key_str = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    let value_str = unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() };
    builder.inner.header(key_str, value_str);
}

/// Set the body of the response.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// lh_response_builder_body(builder, "Hello, world!");
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_body(builder: *mut lh_response_builder_t, body: *const c_char) {
    let builder = unsafe { &mut *builder };
    let body_str = unsafe { CStr::from_ptr(body).to_bytes() };
    builder.inner.body(body_str);
}

/// Write to the body of the response.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// lh_response_builder_body_write(builder, "Hello, world!", 13);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_body_write(builder: *mut lh_response_builder_t, data: *const c_char, len: usize) -> usize {
    let builder = unsafe { &mut *builder };
    let data = unsafe { std::slice::from_raw_parts(data as *const u8, len) };
    builder.inner.body.put(data);
    return len;
}

/// Write to the log of the response.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// lh_response_builder_log_write(builder, "Hello, world!", 13);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_log_write(builder: *mut lh_response_builder_t, data: *const c_char, len: usize) -> usize {
    let builder = unsafe { &mut *builder };
    let data = unsafe { std::slice::from_raw_parts(data as *const u8, len) };
    builder.inner.log.put(data);
    builder.inner.log.put("\n".as_bytes());
    return len;
}

/// Set the exception string of the response.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
/// lh_response_builder_exception(builder, "Something went wrong!");
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_exception(builder: *mut lh_response_builder_t, exception: *const c_char) {
    let builder = unsafe { &mut *builder };
    let exception_str = unsafe { CStr::from_ptr(exception).to_string_lossy().into_owned() };
    builder.inner.exception(exception_str);
}

/// Build a `lh_response_t` from a `lh_response_builder_t`.
///
/// # Examples
///
/// ```c
/// lh_response_builder_t* builder = lh_response_builder_new();
///
/// // Populate the builder data...
///
/// lh_response_t* response = lh_response_builder_build(builder);
/// ```
#[no_mangle]
pub extern "C" fn lh_response_builder_build(builder: *const lh_response_builder_t) -> *mut lh_response_t {
    let builder = unsafe { &*builder };
    Box::into_raw(Box::new(builder.inner.build().into()))
}

/// An HTTP URL. Includes scheme, host, port, domain, origin, authority, username, password, path, query, fragment, and URI.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// ```
#[allow(non_camel_case_types)]
pub struct lh_url_t {
    inner: Url,
}

/// Convert a `Url` into a `lh_url_t`.
impl From<Url> for lh_url_t {
    fn from(inner: Url) -> Self {
        Self { inner }
    }
}

/// Convert a `&lh_url_t` into a `Url`.
impl From<&lh_url_t> for Url {
    fn from(url: &lh_url_t) -> Url {
        url.inner.clone()
    }
}

/// Parse a URL into a `lh_url_t`.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// ```
#[no_mangle]
pub extern "C" fn lh_url_parse(url: *const c_char) -> *mut lh_url_t {
    let url = unsafe { CStr::from_ptr(url).to_string_lossy().into_owned() };
    let url = Url::parse(&url).unwrap();
    Box::into_raw(Box::new(url.into()))
}

/// Free a `lh_url_t`.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
///
/// // Do something...
///
/// lh_url_free(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_free(url: *mut lh_url_t) {
    if !url.is_null() {
        unsafe {
            drop(Box::from_raw(url));
        }
    }
}

/// Get the scheme of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* scheme = lh_url_scheme(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_scheme(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.scheme()).unwrap().into_raw()
}

/// Get the host of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* host = lh_url_host(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_host(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.host_str().unwrap_or("")).unwrap().into_raw()
}

/// Get the port of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// uint16_t port = lh_url_port(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_port(url: *const lh_url_t) -> u16 {
    let url = unsafe { &*url };
    url.inner.port().unwrap_or(0)
}

/// Get the domain of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* domain = lh_url_domain(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_domain(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.domain().unwrap_or("")).unwrap().into_raw()
}

/// Get the origin of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* origin = lh_url_origin(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_origin(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    let origin = match url.inner.origin() {
        url::Origin::Opaque(_) => {
            format!("{}://", url.inner.scheme())
        },
        url::Origin::Tuple(scheme, host, port) => {
            format!("{}://{}:{}", scheme, host, port)
        }
    };
    CString::new(origin.as_str()).unwrap().into_raw()
}

/// Check if the URL has an authority.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// bool has_authority = lh_url_has_authority(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_has_authority(url: *const lh_url_t) -> bool {
    let url = unsafe { &*url };
    url.inner.has_authority()
}

/// Get the authority of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* authority = lh_url_authority(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_authority(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.authority()).unwrap().into_raw()
}

/// Get the username of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* username = lh_url_username(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_username(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.username()).unwrap().into_raw()
}

/// Get the password of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* password = lh_url_password(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_password(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.password().unwrap_or("")).unwrap().into_raw()
}

/// Get the path of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* path = lh_url_path(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_path(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.path()).unwrap().into_raw()
}

/// Get the query of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* query = lh_url_query(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_query(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.query().unwrap_or("")).unwrap().into_raw()
}

/// Get the fragment of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* fragment = lh_url_fragment(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_fragment(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.fragment().unwrap_or("")).unwrap().into_raw()
}

/// Get the URI of the URL.
///
/// # Examples
///
/// ```c
/// lh_url_t* url = lh_url_parse("https://example.com:8080/path/to/resource?query#fragment");
/// const char* uri = lh_url_uri(url);
/// ```
#[no_mangle]
pub extern "C" fn lh_url_uri(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.as_str()).unwrap().into_raw()
}

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Headers(HashMap<String, Vec<String>>);

impl Headers {
    pub fn new() -> Self {
        Headers(HashMap::new())
    }

    pub fn get<K>(&self, key: K) -> Option<&Vec<String>>
    where
        K: AsRef<str>,
    {
        self.0.get(key.as_ref())
    }

    pub fn set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.0
            .entry(key.into())
            .or_insert_with(Vec::new)
            .push(value.into());
    }

    pub fn remove<K>(&mut self, key: K)
    where
        K: AsRef<str>,
    {
        self.0.remove(key.as_ref());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.0.iter()
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &String> {
        self.0.values().flatten()
    }
}

#[allow(non_camel_case_types)]
pub struct lh_headers_t {
    inner: Headers,
}

impl From<Headers> for lh_headers_t {
    fn from(inner: Headers) -> Self {
        Self { inner }
    }
}

impl From<&lh_headers_t> for Headers {
    fn from(headers: &lh_headers_t) -> Headers {
        headers.inner.clone()
    }
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_headers_new() -> *mut lh_headers_t {
    let headers = Headers::new();
    Box::into_raw(Box::new(headers.into()))
}

#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn lh_headers_free(headers: *mut lh_headers_t) {
    if !headers.is_null() {
        unsafe {
            drop(Box::from_raw(headers));
        }
    }
}

#[cfg(feature = "c")]
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

#[cfg(feature = "c")]
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

#[cfg(feature = "c")]
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

#[cfg(feature = "c")]
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

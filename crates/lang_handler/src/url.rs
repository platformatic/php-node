use std::ffi::{CString, c_char};

pub use url::Url;

#[allow(non_camel_case_types)]
pub struct lh_url_t {
    inner: Url,
}

impl From<Url> for lh_url_t {
    fn from(inner: Url) -> Self {
        Self { inner }
    }
}

impl From<&lh_url_t> for Url {
    fn from(url: &lh_url_t) -> Url {
        url.inner.clone()
    }
}

#[no_mangle]
pub extern "C" fn lh_url_scheme(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.scheme()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_host(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.host_str().unwrap_or("")).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_domain(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.domain().unwrap_or("")).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_port(url: *const lh_url_t) -> u16 {
    let url = unsafe { &*url };
    url.inner.port().unwrap_or(0)
}

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

#[no_mangle]
pub extern "C" fn lh_url_has_authority(url: *const lh_url_t) -> bool {
    let url = unsafe { &*url };
    url.inner.has_authority()
}

#[no_mangle]
pub extern "C" fn lh_url_authority(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.authority()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_username(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.username()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_password(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.password().unwrap_or("")).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_path(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.path()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_query(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.query().unwrap_or("")).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_fragment(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.fragment().unwrap_or("")).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lh_url_uri(url: *const lh_url_t) -> *const c_char {
    let url = unsafe { &*url };
    CString::new(url.inner.as_str()).unwrap().into_raw()
}

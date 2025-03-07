mod handler;
mod headers;
mod request;
mod response;
mod url;

use std::ffi::{CString, c_char};

pub use handler::Handler;
pub use headers::{Headers, lh_headers_t};
pub use request::{Request, RequestBuilder, lh_request_t, lh_request_builder_t};
pub use response::{Response, ResponseBuilder, lh_response_t, lh_response_builder_t};
pub use url::{Url, lh_url_t};

#[no_mangle]
pub extern "C" fn lh_reclaim_str(url: *const c_char) {
    unsafe {
        drop(CString::from_raw(url as *mut c_char));
    }
}

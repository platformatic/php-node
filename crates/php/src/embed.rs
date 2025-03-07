use std::{env::Args, ffi::{c_void, c_char, CStr, CString}};

use lang_handler::{Handler, Request, Response};

use crate::sys;

#[derive(Debug, Clone)]
pub struct Embed {
    code: String,
    filename: Option<String>,
}

unsafe impl Send for Embed {}
unsafe impl Sync for Embed {}

impl Embed {
    pub fn new<C, F>(code: C, filename: Option<F>) -> Self
    where
        C: Into<String>,
        F: Into<String>
    {
        Embed::new_with_argv::<C, F, String>(code, filename, vec![])
    }

    pub fn new_with_args<C, F>(code: C, filename: Option<F>, args: Args) -> Self
    where
        C: Into<String>,
        F: Into<String>
    {
        let argv: Vec<String> = args.collect();
        Embed::new_with_argv(code, filename, argv)
    }

    pub fn new_with_argv<C, F, S>(code: C, filename: Option<F>, argv: Vec<S>) -> Self
    where
        C: Into<String>,
        F: Into<String>,
        S: AsRef<str>,
    {
        let argc = argv.len() as i32;
        let argv = argv
            .into_iter()
            .map(|v| CString::new(v.as_ref()).unwrap())
            .collect::<Vec<_>>();

        let mut argv_ptrs = argv
            .iter()
            .map(|v| v.as_ptr() as *mut c_char)
            .collect::<Vec<*mut c_char>>();

        unsafe {
            sys::php_http_init(argc, argv_ptrs.as_mut_ptr());
        }

        Embed {
            code: code.into(),
            filename: filename.map(|v| v.into()),
        }
    }
}

impl Drop for Embed {
    fn drop(&mut self) {
        unsafe {
            sys::php_http_shutdown();
        }
    }
}

impl Handler for Embed {
    type Error = String;

    fn handle(&self, request: Request) -> Result<Response, Self::Error> {
        let code = CString::new(self.code.clone())
            .unwrap();

        let filename = self.filename
            .as_ref()
            .map(|v| CString::new(v.clone()))
            .unwrap_or(CString::new("<unnamed>"))
            .unwrap();

        let mut request: lang_handler::lh_request_t = request.into();
        let request = &mut request as *mut _ as *mut sys::lh_request_t;

        let response = unsafe {
            sys::php_http_handle_request(code.as_ptr(), filename.as_ptr(), request)
        };

        if response.is_null() {
            return Err("Failed to handle request".into());
        }

        let response = unsafe {
            &*(response as *mut lang_handler::lh_response_t)
        };

        Ok(response.into())
    }
}

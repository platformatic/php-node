use std::{env::Args, ffi::{CStr, CString}};

use crate::{sys, Request, Response};

pub struct Embed;

fn args_to_c(args: Args) -> (i32, *mut *mut std::os::raw::c_char) {
    let mut c_args = Vec::new();
    let mut c_ptrs = Vec::new();

    for arg in args {
        let c_arg = std::ffi::CString::new(arg).unwrap();
        let c_ptr = c_arg.clone().into_raw();
        c_args.push(c_arg);
        c_ptrs.push(c_ptr);
    }

    let c_args = c_args
        .into_iter()
        .map(|c_arg| c_arg.into_raw())
        .collect::<Vec<_>>();

    let mut c_ptrs = c_ptrs
        .into_iter()
        .collect::<Vec<_>>();

    (c_args.len() as i32, c_ptrs.as_mut_ptr())
}

impl Embed {
    pub fn new() -> Self {
        Embed::new_with_c_args(0, std::ptr::null_mut())
    }

    pub fn new_with_args(args: Args) -> Self {
        let argv: Vec<String> = args.collect();
        Embed::new_with_argv(argv)
    }

    pub fn new_with_argv<S>(argv: Vec<S>) -> Self
    where
        S: AsRef<str>,
    {
        let mut c_args = Vec::new();
        let mut c_ptrs = Vec::new();

        for arg in argv {
            let c_arg = CString::new(arg.as_ref()).unwrap();
            let c_ptr = c_arg.clone().into_raw();
            c_args.push(c_arg);
            c_ptrs.push(c_ptr);
        }

        let c_args = c_args
            .into_iter()
            .map(|c_arg| c_arg.into_raw())
            .collect::<Vec<_>>();

        let mut c_ptrs = c_ptrs
            .into_iter()
            .collect::<Vec<_>>();

        Embed::new_with_c_args(c_args.len() as i32, c_ptrs.as_mut_ptr())
    }

    fn new_with_c_args(argc: i32, argv: *mut *mut std::os::raw::c_char) -> Self {
        unsafe { sys::php_embed_init(argc, argv); }
        Embed
    }

    pub fn handle_request<C, F>(&self, code: C, filename: Option<F>, request: Request) -> Response
    where
        C: AsRef<str>,
        F: Into<String>,
    {
        let code = CString::new(code.as_ref())
            .unwrap();

        let filename = filename
            .map(|v| CString::new(v.into()))
            .unwrap_or(CString::new("<unnamed>"))
            .unwrap();

        unsafe {
            sys::php_http_handle_request(code.as_ptr(), filename.as_ptr(), *request)
        }.into()
    }
}

impl Drop for Embed {
    fn drop(&mut self) {
        unsafe {
            sys::php_embed_shutdown();
        }
    }
}

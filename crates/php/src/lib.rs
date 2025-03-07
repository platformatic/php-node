#![warn(rust_2018_idioms)]
#![warn(clippy::dbg_macro, clippy::print_stdout)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
// TODO Because `bindgen` generates codes contains deref nullptr, temporary suppression.
#![allow(deref_nullptr)]
#![allow(clippy::all)]
// #![deny(clippy::all)]

mod embed;
mod sys;

pub use lang_handler::{Handler, Headers, Request, Response, RequestBuilder, Url};

pub use embed::Embed;
pub use self::sys::*;

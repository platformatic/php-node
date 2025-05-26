#![warn(rust_2018_idioms)]
#![warn(clippy::dbg_macro, clippy::print_stdout)]

mod embed;
mod exception;
mod request_context;
mod sapi;
mod scopes;
mod strings;

pub use lang_handler::{rewrite, Handler, Header, Headers, Request, RequestBuilder, Response, Url};

pub use embed::Embed;
pub use exception::EmbedException;
pub use request_context::RequestContext;
pub(crate) use sapi::Sapi;
pub(crate) use scopes::RequestScope;

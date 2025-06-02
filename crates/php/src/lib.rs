#![warn(rust_2018_idioms)]
#![warn(clippy::dbg_macro, clippy::print_stdout)]

mod embed;
mod exception;
mod request_context;
mod sapi;
mod scopes;
mod strings;

pub use lang_handler::{Handler, Header, Headers, Request, RequestBuilder, Response, Url};

pub use embed::Embed;
pub use exception::{EmbedRequestError, EmbedStartError};
pub use request_context::RequestContext;

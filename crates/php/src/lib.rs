//! # Embedding PHP in Rust
//!
//! This library implements the PHP SAPI in Rust using Lang Handler, allowing
//! PHP to serve as a handler for HTTP requests dispatched from Rust.
//!
//! ## Example
//!
//! ```rust
//! use std::env::{args, current_dir};
//! # use std::path::PathBuf;
//! # use php::MockRoot;
//! use php::{
//!   rewrite::{PathRewriter, Rewriter},
//!   Embed, Handler, Request,
//! };
//!
//! let docroot = current_dir()
//!   .expect("should have current_dir");
//! # let docroot = MockRoot::builder()
//! #   .file("index.php", "<?php echo \"Hello, World!\"; ?>")
//! #   .build()
//! #   .expect("should prepare docroot");
//!
//! let embed = Embed::new_with_args(docroot, None, args())
//!   .expect("should construct embed");
//!
//! let request = http_handler::request::Request::builder()
//!   .method("POST")
//!   .uri("http://example.com/index.php")
//!   .header("Content-Type", "text/html")
//!   .header("Content-Length", "13")
//!   .body(bytes::BytesMut::from("Hello, World!"))
//!   .expect("should build request");
//!
//! # tokio_test::block_on(async {
//! let response = embed
//!   .handle(request.clone())
//!   .await
//!   .expect("should handle request");
//!
//! assert_eq!(response.status(), 200);
//! assert_eq!(response.body(), "Hello, World!");
//! println!("Response: {:#?}", response);
//! # });
//! ```

#![warn(rust_2018_idioms)]
#![warn(clippy::dbg_macro, clippy::print_stdout)]
#![warn(missing_docs)]

mod embed;
mod exception;
mod request_context;
mod rewriter_impl;
mod sapi;
mod scopes;
mod strings;
mod test;

pub use http_handler::{
    Handler, Request, Response,
    RequestBuilderExt, ResponseExt, ResponseException,
};
pub use http_rewriter as rewrite;

// Re-export commonly used types from http crate
pub use http_handler::{
    HeaderMap as Headers, HeaderName, HeaderValue, Method, StatusCode, Uri as Url,
    header::HeaderName as Header,
};

pub use embed::{Embed, RequestRewriter};
pub use exception::{EmbedRequestError, EmbedStartError};
pub use request_context::RequestContext;
pub use rewriter_impl::*;
pub use test::{MockRoot, MockRootBuilder};

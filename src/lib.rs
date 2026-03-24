//! # Embedding PHP in Rust
//!
//! This library implements the PHP SAPI in Rust using Lang Handler, allowing
//! PHP to serve as a handler for HTTP requests dispatched from Rust.
//!
//! ## Example
//!
//! ```rust,no_run
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
//! # tokio_test::block_on(async {
//! let body = http_handler::RequestBody::new();
//!
//! // Write body data and close the stream
//! {
//!   use tokio::io::AsyncWriteExt;
//!   let mut body_writer = body.clone();
//!   body_writer.write_all(b"Hello, World!").await.expect("should write body");
//!   body_writer.shutdown().await.expect("should close request body stream");
//! }
//!
//! let request = http_handler::request::Request::builder()
//!   .method("POST")
//!   .uri("http://example.com/index.php")
//!   .header("Content-Type", "text/html")
//!   .header("Content-Length", "13")
//!   .body(body)
//!   .expect("should build request");
//!
//! let response = embed
//!   .handle(request.clone())
//!   .await
//!   .expect("should handle request");
//!
//! assert_eq!(response.status(), 200);
//!
//! // Consume the streaming response body to ensure PHP task completes
//! use http_body_util::BodyExt;
//! let (_parts, body) = response.into_parts();
//! let mut stream = body;
//! while let Some(frame_result) = stream.frame().await {
//!   match frame_result {
//!     Ok(_) => continue,
//!     Err(e) => panic!("Error reading response: {}", e),
//!   }
//! }
//!
//! drop(embed);
//! # });
//! ```

#![warn(rust_2018_idioms)]
#![warn(clippy::dbg_macro, clippy::print_stdout)]
#![warn(missing_docs)]

#[cfg(feature = "napi-support")]
#[macro_use]
extern crate napi_derive;

mod embed;
mod exception;
mod extensions;
mod request_context;
mod sapi;
mod scopes;
mod strings;
mod test;

#[cfg(feature = "napi-support")]
/// NAPI bindings for exposing PHP to Node.js
pub mod napi;

pub use http_handler::types::{Request, Response};
pub use http_handler::{Handler, RequestBuilderExt, ResponseException, ResponseExt};
pub use http_rewriter as rewrite;

// Re-export commonly used types from http crate
pub use http_handler::{
  header::HeaderName as Header, HeaderMap as Headers, HeaderName, HeaderValue, Method, StatusCode,
  Uri as Url,
};

pub use embed::{Embed, RequestRewriter};
pub use exception::{EmbedRequestError, EmbedStartError};
pub use extensions::{HeadersSentTx, RequestStream, ResponseStream};
pub use request_context::RequestContext;
pub use test::{MockRoot, MockRootBuilder};

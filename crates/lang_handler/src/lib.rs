#![warn(missing_docs)]

//! # HTTP Request Management
//!
//! Lang Handler is a library intended for managing HTTP requests between
//! multiple languages. It provides types for representing Headers, Request,
//! and Response, as well as providing a Handler trait for dispatching
//! Request objects into some other system which produces a Response.
//! This may be another language runtime, or it could be a server application
//! directly in Rust.
//!
//! # Building a Request
//!
//! The `Request` type provides a `builder` method which allows you to
//! construct a `Request` object using a fluent API. This allows you to
//! set the URL, HTTP method, headers, body, and other properties of the
//! request in a clear and concise manner.
//!
//! ```rust
//! use lang_handler::{Request, RequestBuilder};
//!
//! let request = Request::builder()
//!   .method("GET")
//!   .url("http://example.com")
//!   .header("Accept", "application/json")
//!   .build()
//!   .expect("should build request");
//! ```
//!
//! # Reading a Request
//!
//! The `Request` type also provides methods to read the [`Url`], HTTP method,
//! [`Headers`], and body of the request. This allows you to access the
//! properties of the request in a straightforward manner.
//!
//! ```rust
//! # use lang_handler::Request;
//! #
//! # let request = Request::builder()
//! #   .method("GET")
//! #   .url("http://example.com")
//! #   .header("Accept", "application/json")
//! #   .build()
//! #   .expect("should build request");
//! #
//! assert_eq!(request.method(), "GET");
//! assert_eq!(request.url().to_string(), "http://example.com/");
//! assert_eq!(request.headers().get("Accept"), Some("application/json".to_string()));
//! assert_eq!(request.body(), "");
//! ```
//!
//! # Building a Response
//!
//! The `Response` type also provides a `builder` method which allows you to
//! construct a `Response` object using a fluent API. This allows you to
//! set the status code, [`Headers`], body, and other properties of the
//! response in a clear and concise manner.
//!
//! ```rust
//! use lang_handler::{Response, ResponseBuilder};
//!
//! let response = Response::builder()
//!   .status(200)
//!   .header("Content-Type", "application/json")
//!   .body("{\"message\": \"Hello, world!\"}")
//!   .log("This is a log message")
//!   .exception("This is an exception message")
//!   .build();
//! ```
//!
//! # Reading a Response
//!
//! The `Response` type provides methods to read the status code, [`Headers`],
//! body, log, and exception of the response.
//!
//! ```rust
//! # use lang_handler::{Response, ResponseBuilder};
//! #
//! # let response = Response::builder()
//! #   .status(200)
//! #   .header("Content-Type", "text/plain")
//! #   .body("Hello, World!")
//! #   .log("This is a log message")
//! #   .exception("This is an exception message")
//! #   .build();
//! #
//! assert_eq!(response.status(), 200);
//! assert_eq!(response.headers().get("Content-Type"), Some("text/plain".to_string()));
//! assert_eq!(response.body(), "Hello, World!");
//! assert_eq!(response.log(), "This is a log message");
//! assert_eq!(response.exception(), Some(&"This is an exception message".to_string()));
//! ```
//! # Managing Headers
//!
//! The `Headers` type provides methods to read and manipulate HTTP headers.
//!
//! ```rust
//! use lang_handler::Headers;
//!
//! // Setting and getting headers
//! let mut headers = Headers::new();
//! headers.set("Content-Type", "application/json");
//! assert_eq!(headers.get("Content-Type"), Some("application/json".to_string()));
//!
//! // Checking if a header exists
//! assert!(headers.has("Content-Type"));
//!
//! // Removing headers
//! headers.remove("Content-Type");
//! assert_eq!(headers.get("Content-Type"), None);
//!
//! // Adding multiple values to a header
//! headers.add("Set-Cookie", "sessionid=abc123");
//! headers.add("Set-Cookie", "userid=42");
//!
//! // Iterating over headers
//! for (name, value) in headers.iter() {
//!   println!("{}: {:?}", name, value);
//! }
//!
//! // Getting all values for a header
//! let cookies = headers.get_all("Set-Cookie");
//! assert_eq!(cookies, vec!["sessionid=abc123", "userid=42"]);
//!
//! // Getting a set of headers as a string line
//! headers.add("Accept", "text/plain");
//! headers.add("Accept", "application/json");
//! let accept_header = headers.get_line("Accept");
//!
//! // Counting header lines
//! assert!(headers.len() > 0);
//!
//! // Clearing all headers
//! headers.clear();
//!
//! // Checking if headers are empty
//! assert!(headers.is_empty());
//! ```
//! # Handling Requests
//!
//! The `Handler` trait is used to define how a [`Request`] is handled. It
//! provides a method `handle` which takes a [`Request`] and returns a
//! [`Response`]. This allows you to implement custom logic for handling
//! requests, such as routing them to different services or processing them
//! in some way.
//!
//! ```rust
//! use lang_handler::{
//!   Handler,
//!   Request,
//!   RequestBuilder,
//!   Response,
//!   ResponseBuilder
//! };
//!
//! pub struct EchoServer;
//! impl Handler for EchoServer {
//!   type Error = String;
//!   fn handle(&self, request: Request) -> Result<Response, Self::Error> {
//!     let response = Response::builder()
//!       .status(200)
//!       .body(request.body())
//!       .build();
//!
//!     Ok(response)
//!   }
//! }
//!
//! let handler = EchoServer;
//!
//! let request = Request::builder()
//!   .method("POST")
//!   .url("http://example.com")
//!   .header("Accept", "application/json")
//!   .body("Hello, world!")
//!   .build()
//!   .expect("should build request");
//!
//! let response = handler.handle(request)
//!   .expect("should handle request");
//!
//! assert_eq!(response.status(), 200);
//! assert_eq!(response.body(), "Hello, world!");
//! ```

#[cfg(feature = "c")]
mod ffi;
mod handler;
mod headers;
mod request;
mod response;
pub mod rewrite;
mod test;

#[cfg(feature = "c")]
pub use ffi::*;
pub use handler::Handler;
pub use headers::{Header, Headers};
pub use request::{Request, RequestBuilder, RequestBuilderException};
pub use response::{Response, ResponseBuilder};
pub use test::{MockRoot, MockRootBuilder};
pub use url::Url;

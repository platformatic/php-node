/// Custom extension types for RequestWrapper
///
/// These extensions store request-specific state that needs to be shared
/// across SAPI callbacks and async boundaries.
use bytes::Bytes;
use http_handler::ResponseBody;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

/// Extension for storing the response body stream
///
/// Allows SAPI callbacks to write response body data.
#[derive(Clone)]
pub struct ResponseStream(pub ResponseBody);

impl ResponseStream {
  /// Create a new ResponseStream extension with the given response body
  pub fn new(body: ResponseBody) -> Self {
    Self(body)
  }
}

/// Extension for storing the headers sent notification channel
///
/// Used to signal when headers are finalized and send them to the response builder.
/// Contains: (status_code, mime_type, custom_headers, logs)
#[derive(Clone)]
pub struct HeadersSentTx(
  pub Arc<Mutex<Option<oneshot::Sender<(u16, String, Vec<(String, String)>, Bytes)>>>>,
);

impl HeadersSentTx {
  /// Create a new HeadersSentTx extension with the given oneshot sender
  pub fn new(sender: oneshot::Sender<(u16, String, Vec<(String, String)>, Bytes)>) -> Self {
    Self(Arc::new(Mutex::new(Some(sender))))
  }
}

/// Extension for storing the request body stream
///
/// Allows SAPI callbacks to read streaming request body data.
#[derive(Clone)]
pub struct RequestStream(pub http_handler::RequestBody);

impl RequestStream {
  /// Create a new RequestStream extension with the given request body
  pub fn new(body: http_handler::RequestBody) -> Self {
    Self(body)
  }
}

/// Request wrapper for the PHP SAPI
///
/// This wrapper stores the minimal non-Clone state that cannot be moved to extensions:
/// - The Request itself (for Deref access)
/// - The response Builder (non-Clone, cannot use std::mem::replace pattern)
/// - The request Receiver (non-Clone, single ownership)
///
/// All shareable state is stored in Request extensions:
/// - DocumentRoot (http-handler) - docroot path
/// - ResponseLog (http-handler) - log buffer
/// - BodyBuffer (http-handler) - request body buffer
/// - ResponseStream (custom) - response body stream
/// - RequestStream (custom) - request body stream
/// - HeadersSentTx (custom) - headers sent notification
use bytes::Bytes;
use ext_php_rs::zend::SapiGlobals;
use http_handler::extensions::{BodyBuffer, DocumentRoot, ResponseLog};
use http_handler::types::Request;
use http_handler::RequestExt;
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use tokio::sync::oneshot;

use crate::extensions::{HeadersSentTx, RequestStream, ResponseStream};

/// The request context for the PHP SAPI.
///
/// This is a minimal wrapper around Request that provides Deref/DerefMut access.
/// All state is stored in Request extensions.
pub struct RequestContext(Request);

impl Deref for RequestContext {
  type Target = Request;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for RequestContext {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl RequestContext {
  /// Creates a new RequestContext for handling a PHP request.
  ///
  /// Since SAPI is always in streaming mode (buffering happens at the NAPI Task layer),
  /// we always set up streaming channels. The docroot and all state are
  /// stored as Request extensions.
  pub fn new<P>(
    mut request: Request,
    docroot: P,
    response_body: http_handler::ResponseBody,
    headers_sent_tx: oneshot::Sender<(u16, String, Vec<(String, String)>, Bytes)>,
  ) -> Self
  where
    P: AsRef<Path>,
  {
    // Get the request body for SAPI to read from
    let request_body = request.body().clone();

    // Store all state in extensions
    request.set_document_root(DocumentRoot {
      path: docroot.as_ref().to_path_buf(),
    });

    request.extensions_mut().insert(ResponseLog::new());

    request.extensions_mut().insert(BodyBuffer::new());

    request
      .extensions_mut()
      .insert(ResponseStream::new(response_body));

    request
      .extensions_mut()
      .insert(HeadersSentTx::new(headers_sent_tx));

    request
      .extensions_mut()
      .insert(RequestStream::new(request_body));

    Self(request)
  }

  /// Sets the current request context for the PHP SAPI.
  pub fn set_current(context: Box<RequestContext>) {
    let mut globals = SapiGlobals::get_mut();
    globals.server_context = Box::into_raw(context) as *mut c_void;
  }

  /// Retrieve a mutable reference to the request context
  pub fn current<'a>() -> Option<&'a mut RequestContext> {
    let ptr = {
      let globals = SapiGlobals::get();
      globals.server_context as *mut RequestContext
    };
    if ptr.is_null() {
      return None;
    }

    Some(unsafe { &mut *ptr })
  }

  /// Reclaim ownership of the RequestContext.
  ///
  /// This also shuts down the response stream to signal EOF to response body consumers.
  /// The shutdown blocks until complete to avoid use-after-free issues where the async
  /// task could access memory after RequestContext is dropped.
  #[allow(dead_code)]
  pub fn reclaim() -> Option<Box<RequestContext>> {
    // First shutdown the response stream while context is still valid
    if let Some(ctx) = Self::current() {
      ctx.shutdown_response_stream();
    }

    let ptr = {
      let mut globals = SapiGlobals::get_mut();
      std::mem::replace(&mut globals.server_context, std::ptr::null_mut())
    };
    if ptr.is_null() {
      return None;
    }
    Some(unsafe { Box::from_raw(ptr as *mut RequestContext) })
  }

  /// Signal that headers have been sent with status, mimetype, headers, and logs (streaming mode).
  pub fn signal_headers_sent_with_data(
    &mut self,
    status: u16,
    mimetype: String,
    headers: Vec<(String, String)>,
    logs: Bytes,
  ) {
    if let Some(headers_sent_tx) = self.extensions_mut().get_mut::<HeadersSentTx>() {
      if let Ok(mut guard) = headers_sent_tx.0.try_lock() {
        if let Some(tx) = guard.take() {
          let _ = tx.send((status, mimetype, headers, logs));
        }
      }
    }
  }

  /// Shutdown the response stream to signal EOF to response body consumers.
  /// This blocks until the shutdown is complete to avoid use-after-free.
  pub fn shutdown_response_stream(&mut self) {
    use tokio::io::AsyncWriteExt;

    if let Some(response_stream) = self.extensions().get::<ResponseStream>() {
      let mut body = response_stream.0.clone();
      // IMPORTANT: We must wait for shutdown to complete before returning.
      // Previously this was spawned without waiting, which caused use-after-free:
      // the async task could access memory after RequestContext was dropped.
      crate::sapi::fallback_handle().block_on(async move {
        let _ = body.shutdown().await;
      });
    }
  }
}

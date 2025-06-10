use lang_handler::RequestBuilderException;

/// Set of exceptions which may be produced by php::Embed
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum EmbedStartError {
  /// Document root not found, or not a directory
  DocRootNotFound(String),

  /// Failed to identify the executable location
  ExeLocationNotFound,

  /// Failed to initialize SAPI
  SapiNotInitialized,
}

impl std::fmt::Display for EmbedStartError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EmbedStartError::DocRootNotFound(docroot) => {
        write!(f, "Document root not found: {}", docroot)
      }
      EmbedStartError::ExeLocationNotFound => {
        write!(f, "Failed to identify executable location")
      }
      EmbedStartError::SapiNotInitialized => write!(f, "Failed to initialize SAPI"),
    }
  }
}

/// Errors which may occur during the request lifecycle
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum EmbedRequestError {
  /// SAPI not started
  SapiNotStarted,

  /// SAPI not shutdown
  SapiNotShutdown,

  /// SAPI request not started
  SapiRequestNotStarted,

  /// Request context unavailable
  RequestContextUnavailable,

  /// Failed to encode a string to a C-style string
  CStringEncodeFailed(String),

  // ExecuteError,
  /// Exception thrown during script execution
  Exception(String),

  /// PHP bailout, usually due to a fatal error or exit call
  Bailout,

  /// Failed to build the response
  ResponseBuildError,

  /// Failed to find the current directory
  FailedToFindCurrentDirectory,

  /// Expected an absolute REQUEST_URI, but received a relative one
  ExpectedAbsoluteRequestUri(String),

  /// Script not found in the document root
  ScriptNotFound(String),

  /// Failed to determine the content type of the response
  FailedToDetermineContentType,

  /// Failed to set a server variable
  FailedToSetServerVar(String),

  /// Failed to set request info
  FailedToSetRequestInfo(String),

  /// Error during request rewriting
  RequestRewriteError(RequestBuilderException),
}

impl std::fmt::Display for EmbedRequestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EmbedRequestError::SapiNotStarted => write!(f, "Failed to start SAPI"),
      EmbedRequestError::SapiNotShutdown => write!(f, "Failed to shutdown SAPI"),
      EmbedRequestError::RequestContextUnavailable => write!(f, "Request context unavailable"),
      EmbedRequestError::SapiRequestNotStarted => write!(f, "Failed to start SAPI request"),
      EmbedRequestError::CStringEncodeFailed(e) => {
        write!(f, "Failed to encode to cstring: \"{}\"", e)
      }
      // EmbedRequestError::ExecuteError => write!(f, "Script execution error"),
      EmbedRequestError::Exception(e) => write!(f, "Exception thrown: {}", e),
      EmbedRequestError::Bailout => write!(f, "PHP bailout"),
      EmbedRequestError::ResponseBuildError => write!(f, "Failed to build response"),
      EmbedRequestError::FailedToFindCurrentDirectory => {
        write!(f, "Failed to identify current directory")
      }
      EmbedRequestError::ExpectedAbsoluteRequestUri(e) => {
        write!(f, "Expected absolute REQUEST_URI: {}", e)
      }
      EmbedRequestError::ScriptNotFound(e) => write!(f, "Script not found: {}", e),
      EmbedRequestError::FailedToDetermineContentType => {
        write!(f, "Failed to determine content type")
      }
      EmbedRequestError::FailedToSetServerVar(name) => {
        write!(f, "Failed to set server var: \"{}\"", name)
      }
      EmbedRequestError::FailedToSetRequestInfo(name) => {
        write!(f, "Failed to set request info: \"{}\"", name)
      }
      EmbedRequestError::RequestRewriteError(e) => {
        write!(f, "Request rewrite error: {}", e)
      }
    }
  }
}

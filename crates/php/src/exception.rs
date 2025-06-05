/// Set of exceptions which may be produced by php::Embed
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum EmbedStartError {
  DocRootNotFound(String),
  ExeLocationNotFound,
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum EmbedRequestError {
  SapiNotStarted,
  SapiNotShutdown,
  SapiRequestNotStarted,
  RequestContextUnavailable,
  CStringEncodeFailed(String),
  // ExecuteError,
  Exception(String),
  Bailout,
  ResponseBuildError,
  FailedToFindCurrentDirectory,
  ExpectedAbsoluteRequestUri(String),
  ScriptNotFound(String),
  FailedToDetermineContentType,
  FailedToSetServerVar(String),
  FailedToSetRequestInfo(String),
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
    }
  }
}

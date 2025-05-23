/// Set of exceptions which may be produced by php::Embed
#[derive(Debug)]
pub enum EmbedException {
  DocRootNotFound(String),
  SapiNotInitialized,
  SapiNotStarted,
  SapiLockFailed,
  SapiMissingStartupFunction,
  FailedToFindExeLocation,
  SapiRequestNotStarted,
  RequestContextUnavailable,
  CStringEncodeFailed(String),
  CStringDecodeFailed(usize),
  HeaderNotFound(String),
  // ExecuteError,
  Exception(String),
  Bailout,
  ResponseBuildError,
  FailedToFindCurrentDirectory,
  ExpectedAbsoluteRequestUri(String),
  ScriptNotFound(String),
}

impl std::fmt::Display for EmbedException {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EmbedException::RequestContextUnavailable => write!(f, "Request context unavailable"),
      EmbedException::SapiNotInitialized => write!(f, "Failed to initialize SAPI"),
      EmbedException::SapiLockFailed => write!(f, "Failed to acquire SAPI lock"),
      EmbedException::SapiMissingStartupFunction => write!(f, "Missing SAPI startup function"),
      EmbedException::FailedToFindExeLocation => write!(f, "Failed to identify executable location"),
      EmbedException::DocRootNotFound(docroot) => write!(f, "Document root not found: {}", docroot),
      EmbedException::SapiNotStarted => write!(f, "Failed to start SAPI"),
      EmbedException::SapiRequestNotStarted => write!(f, "Failed to start SAPI request"),
      EmbedException::CStringEncodeFailed(e) => write!(f, "Failed to encode to cstring: \"{}\"", e),
      EmbedException::CStringDecodeFailed(e) => write!(f, "Failed to decode from cstring: {}", e),
      EmbedException::HeaderNotFound(header) => write!(f, "Header not found: {}", header),
      // EmbedException::ExecuteError => write!(f, "Script execution error"),
      EmbedException::Exception(e) => write!(f, "Exception thrown: {}", e),
      EmbedException::Bailout => write!(f, "PHP bailout"),
      EmbedException::ResponseBuildError => write!(f, "Failed to build response"),
      EmbedException::FailedToFindCurrentDirectory => write!(f, "Failed to identify current directory"),
      EmbedException::ExpectedAbsoluteRequestUri(e) => write!(f, "Expected absolute REQUEST_URI: {}", e),
      EmbedException::ScriptNotFound(e) => write!(f, "Script not found: {}", e),
    }
  }
}

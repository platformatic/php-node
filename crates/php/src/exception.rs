use std::{ffi::NulError, path::StripPrefixError};

/// Set of exceptions which may be produced by php::Embed
#[derive(Debug)]
pub enum EmbedException {
  DocRootNotFound(String),
  SapiNotInitialized,
  SapiStartupError,
  SapiLockFailed,
  SapiMissingStartupFunction,
  ExeLocationError,
  RequestStartupError,
  RequestContextUnavailable,
  InvalidCString(NulError),
  InvalidStr(std::str::Utf8Error),
  HeaderNotFound(String),
  ExecuteError,
  Exception(String),
  Bailout,
  ResponseError,
  IoError(std::io::Error),
  RelativizeError(StripPrefixError),
  CanonicalizeError(std::io::Error),
}

impl std::fmt::Display for EmbedException {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EmbedException::RequestContextUnavailable => write!(f, "Request context unavailable"),
      EmbedException::SapiNotInitialized => write!(f, "SAPI has not been initialized"),
      EmbedException::SapiLockFailed => write!(f, "Failed to acquire SAPI lock"),
      EmbedException::SapiMissingStartupFunction => write!(f, "Missing SAPI startup function"),
      EmbedException::ExeLocationError => write!(f, "Error getting executable location"),
      EmbedException::DocRootNotFound(docroot) => write!(f, "Document root not found: {}", docroot),
      EmbedException::SapiStartupError => write!(f, "SAPI startup error"),
      EmbedException::RequestStartupError => write!(f, "Request startup error"),
      EmbedException::InvalidCString(e) => write!(f, "CString conversion error: {}", e),
      EmbedException::InvalidStr(e) => write!(f, "String conversion error: {}", e),
      EmbedException::HeaderNotFound(header) => write!(f, "Header not found: {}", header),
      EmbedException::ExecuteError => write!(f, "Script execution error"),
      EmbedException::Exception(e) => write!(f, "Exception thrown: {}", e),
      EmbedException::Bailout => write!(f, "PHP bailout"),
      EmbedException::ResponseError => write!(f, "Error building response"),
      EmbedException::IoError(e) => write!(f, "IO error: {}", e),
      EmbedException::RelativizeError(e) => write!(f, "Path relativization error: {}", e),
      EmbedException::CanonicalizeError(e) => write!(f, "Path canonicalization error: {}", e),
    }
  }
}

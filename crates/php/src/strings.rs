use std::{
  ffi::{c_char, CString},
  path::{Path, PathBuf},
};

use crate::EmbedRequestError;

#[allow(dead_code)]
pub(crate) fn default_cstr<S, V>(
  default: S,
  maybe: Option<V>,
) -> Result<*mut c_char, EmbedRequestError>
where
  S: Into<String>,
  V: Into<String>,
{
  cstr(match maybe {
    Some(v) => v.into(),
    None => default.into(),
  })
}

pub(crate) fn nullable_cstr<S>(maybe: Option<S>) -> Result<*mut c_char, EmbedRequestError>
where
  S: Into<String>,
{
  match maybe {
    Some(v) => cstr(v.into()),
    None => Ok(std::ptr::null_mut()),
  }
}

pub(crate) fn cstr<S: AsRef<str>>(s: S) -> Result<*mut c_char, EmbedRequestError> {
  CString::new(s.as_ref())
    .map_err(|_| EmbedRequestError::CStringEncodeFailed(s.as_ref().to_owned()))
    .map(|cstr| cstr.into_raw())
}

#[allow(dead_code)]
pub(crate) fn reclaim_str(ptr: *mut c_char) -> CString {
  unsafe { CString::from_raw(ptr) }
}

#[allow(dead_code)]
pub(crate) fn drop_str(ptr: *mut c_char) {
  if ptr.is_null() {
    return;
  }
  drop(reclaim_str(ptr));
}

pub(crate) fn translate_path<D, P>(docroot: D, request_uri: P) -> Result<PathBuf, EmbedRequestError>
where
  D: AsRef<Path>,
  P: AsRef<Path>,
{
  let docroot = docroot.as_ref().to_path_buf();
  let request_uri = request_uri.as_ref();
  let relative_uri = request_uri.strip_prefix("/").map_err(|_| {
    let uri = request_uri.display().to_string();
    EmbedRequestError::ExpectedAbsoluteRequestUri(uri)
  })?;

  let exact = docroot.join(relative_uri);

  exact.join("index.php").canonicalize().or_else(|_| {
    exact
      .canonicalize()
      .map_err(|_| EmbedRequestError::ScriptNotFound(exact.display().to_string()))
  })
}

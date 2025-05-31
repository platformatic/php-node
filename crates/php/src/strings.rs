use std::{
  env::current_dir,
  ffi::{c_char, CStr, CString},
  path::{Path, PathBuf},
};

use crate::EmbedException;

#[allow(dead_code)]
pub(crate) fn default_cstr<S: Into<String>, V: Into<String>>(
  default: S,
  maybe: Option<V>,
) -> Result<*mut c_char, EmbedException> {
  cstr(match maybe {
    Some(v) => v.into(),
    None => default.into(),
  })
}

pub(crate) fn nullable_cstr<S: Into<String>>(
  maybe: Option<S>,
) -> Result<*mut c_char, EmbedException> {
  match maybe {
    Some(v) => cstr(v.into()),
    None => Ok(std::ptr::null_mut()),
  }
}

pub(crate) fn cstr<S: AsRef<str>>(s: S) -> Result<*mut c_char, EmbedException> {
  CString::new(s.as_ref())
    .map_err(|_| EmbedException::CStringEncodeFailed(s.as_ref().to_owned()))
    .map(|cstr| cstr.into_raw())
}

pub(crate) fn str_from_cstr<'a>(ptr: *mut c_char) -> Result<&'a str, EmbedException> {
  unsafe { CStr::from_ptr(ptr) }
    .to_str()
    .map_err(|_| EmbedException::CStringDecodeFailed(ptr.addr()))
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

pub(crate) fn maybe_current_dir() -> Result<PathBuf, EmbedException> {
  current_dir()
    .and_then(|dir| dir.canonicalize())
    .or(Err(EmbedException::FailedToFindCurrentDirectory))
}

pub(crate) fn translate_path<D, P>(docroot: D, request_uri: P) -> Result<PathBuf, EmbedException>
where
  D: AsRef<Path>,
  P: AsRef<Path>,
{
  let docroot = docroot.as_ref().to_path_buf();
  let request_uri = request_uri.as_ref();
  let relative_uri = request_uri.strip_prefix("/").map_err(|_| {
    let uri = request_uri.display().to_string();
    EmbedException::ExpectedAbsoluteRequestUri(uri)
  })?;

  let exact = docroot.join(relative_uri);

  exact.join("index.php").canonicalize().or_else(|_| {
    exact
      .canonicalize()
      .map_err(|_| EmbedException::ScriptNotFound(exact.display().to_string()))
  })
}

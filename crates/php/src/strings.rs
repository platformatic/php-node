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

  // NOTE: String conversion is necessary. If Path::ends_with("/") is used it
  // will discard the trailing slash first.
  if request_uri.display().to_string().ends_with("/") {
    try_path(exact.join("index.php")).or_else(|_| try_path(exact))
  } else {
    try_path(exact)
  }
}

fn try_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, EmbedRequestError> {
  let path = path.as_ref();
  let true_path = path
    .canonicalize()
    .map_err(|_| EmbedRequestError::ScriptNotFound(path.display().to_string()))?;

  if true_path.is_file() {
    Ok(true_path)
  } else {
    Err(EmbedRequestError::ScriptNotFound(
      path.display().to_string(),
    ))
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::MockRoot;

  #[test]
  fn test_translate_path() {
    let docroot = MockRoot::builder()
      .file("/index.php", "<?php echo \"index\"; ?>")
      .file("/foo/index.php", "<?php echo \"sub\"; ?>")
      .build()
      .expect("should prepare docroot");

    assert_eq!(
      translate_path(docroot.clone(), "/foo/"),
      Ok(docroot.join("foo/index.php"))
    );
    assert_eq!(
      translate_path(docroot.clone(), "/foo"),
      Err(EmbedRequestError::ScriptNotFound(
        docroot.join("foo").display().to_string()
      ))
    );
  }
}

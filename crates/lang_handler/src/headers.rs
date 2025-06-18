use std::collections::{hash_map::Entry, HashMap};

/// Represents a single HTTP header value or multiple values for the same header.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Header {
  /// A single value for a header.
  Single(String),

  /// Multiple values for a header, stored as a vector.
  Multiple(Vec<String>),
}

impl From<&Header> for String {
  fn from(header: &Header) -> String {
    match header {
      Header::Single(value) => value.to_owned(),
      Header::Multiple(values) => values.join(","),
    }
  }
}

// TODO: Figure out why From<Into<String>> conflicts with From<Vec<String>>
impl From<String> for Header {
  fn from(value: String) -> Header {
    Header::Single(value)
  }
}

impl From<&str> for Header {
  fn from(value: &str) -> Header {
    Header::Single(value.to_string())
  }
}

impl From<Vec<String>> for Header {
  fn from(values: Vec<String>) -> Header {
    Header::Multiple(values)
  }
}

#[cfg(feature = "napi")]
mod napi_header {
  use super::*;

  use napi::bindgen_prelude::*;
  use napi::sys;

  impl FromNapiValue for Header {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> Result<Self> {
      let mut header_type = sys::ValueType::napi_undefined;
      unsafe { check_status!(sys::napi_typeof(env, value, &mut header_type)) }?;

      let header_type: ValueType = header_type.into();

      match header_type {
        ValueType::String => {
          let s = String::from_napi_value(env, value)?;
          Ok(Header::Single(s))
        }
        ValueType::Object => {
          let obj = Vec::<String>::from_napi_value(env, value)?;
          Ok(Header::Multiple(obj))
        }
        _ => Err(napi::Error::new(
          Status::InvalidArg,
          "Expected a string or an object for Header",
        )),
      }
    }
  }

  impl ToNapiValue for Header {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> Result<sys::napi_value> {
      match value {
        Header::Single(value) => String::to_napi_value(env, value),
        Header::Multiple(values) => Vec::<String>::to_napi_value(env, values),
      }
    }
  }
}

/// A multi-map of HTTP headers.
///
/// # Examples
///
/// ```
/// # use lang_handler::Headers;
/// let mut headers = Headers::new();
/// headers.set("Content-Type", "text/plain");
///
/// assert_eq!(headers.get("Content-Type"), Some("text/plain".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct Headers(HashMap<String, Header>);

impl Headers {
  /// Creates a new `Headers` instance.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let headers = Headers::new();
  /// ```
  pub fn new() -> Self {
    Headers(HashMap::new())
  }

  /// Checks if a header field exists.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.set("Content-Type", "text/plain");
  ///
  /// assert!(headers.has("Content-Type"));
  /// assert!(!headers.has("Accept"));
  /// ```
  pub fn has<K>(&self, key: K) -> bool
  where
    K: AsRef<str>,
  {
    self.0.contains_key(key.as_ref().to_lowercase().as_str())
  }

  /// Returns the last single value associated with a header field.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.add("Accept", "text/plain");
  /// headers.add("Accept", "application/json");
  ///
  /// assert_eq!(headers.get("Accept"), Some("application/json".to_string()));
  /// ```
  pub fn get<K>(&self, key: K) -> Option<String>
  where
    K: AsRef<str>,
  {
    match self.0.get(key.as_ref().to_lowercase().as_str()) {
      Some(Header::Single(value)) => Some(value.clone()),
      Some(Header::Multiple(values)) => values.last().cloned(),
      None => None,
    }
  }

  /// Returns all values associated with a header field as a `Vec<String>`.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.add("Accept", "text/plain");
  /// headers.add("Accept", "application/json");
  ///
  /// assert_eq!(headers.get_all("Accept"), vec![
  ///   "text/plain".to_string(),
  ///   "application/json".to_string()
  /// ]);
  ///
  /// headers.set("Content-Type", "text/plain");
  /// assert_eq!(headers.get_all("Content-Type"), vec!["text/plain".to_string()]);
  /// ```
  pub fn get_all<K>(&self, key: K) -> Vec<String>
  where
    K: AsRef<str>,
  {
    match self.0.get(key.as_ref().to_lowercase().as_str()) {
      Some(Header::Single(value)) => vec![value.clone()],
      Some(Header::Multiple(values)) => values.clone(),
      None => Vec::new(),
    }
  }

  /// Returns all values associated with a header field as a single
  /// comma-separated string.
  ///
  /// # Note
  ///
  /// Some headers support delivery as a comma-separated list of values,
  /// but most require multiple header lines to send multiple values.
  /// Typically you should use `get_all` rather than `get_line` and send
  /// multiple header lines.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.add("Accept", "text/plain");
  /// headers.add("Accept", "application/json");
  ///
  /// assert_eq!(headers.get_line("Accept"), Some("text/plain,application/json".to_string()));
  /// ```
  pub fn get_line<K>(&self, key: K) -> Option<String>
  where
    K: AsRef<str>,
  {
    self
      .0
      .get(key.as_ref().to_lowercase().as_str())
      .map(|v| v.into())
  }

  /// Sets a header field, replacing any existing values.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.set("Content-Type", "text/plain");
  /// headers.set("Content-Type", "text/html");
  ///
  /// assert_eq!(headers.get("Content-Type"), Some("text/html".to_string()));
  /// ```
  pub fn set<K, V>(&mut self, key: K, value: V)
  where
    K: Into<String>,
    V: Into<Header>,
  {
    self.0.insert(key.into().to_lowercase(), value.into());
  }

  /// Add a header with the given value without replacing existing ones.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.add("Accept", "text/plain");
  /// headers.add("Accept", "application/json");
  ///
  /// assert_eq!(headers.get_all("Accept"), vec![
  ///   "text/plain".to_string(),
  ///   "application/json".to_string()
  /// ]);
  /// ```
  pub fn add<K, V>(&mut self, key: K, value: V)
  where
    K: Into<String>,
    V: Into<String>,
  {
    let key = key.into().to_lowercase();
    let value = value.into();

    match self.0.entry(key) {
      Entry::Vacant(e) => {
        e.insert(Header::Single(value));
      }
      Entry::Occupied(mut e) => {
        let header = e.get_mut();
        *header = match header {
          Header::Single(existing_value) => {
            let mut values = vec![existing_value.clone()];
            values.push(value);
            Header::Multiple(values)
          }
          Header::Multiple(values) => {
            values.push(value);
            Header::Multiple(values.clone())
          }
        };
      }
    }
  }

  /// Removes a header field.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.set("Content-Type", "text/plain");
  /// headers.remove("Content-Type");
  ///
  /// assert_eq!(headers.get("Content-Type"), None);
  /// ```
  pub fn remove<K>(&mut self, key: K)
  where
    K: AsRef<str>,
  {
    self.0.remove(key.as_ref().to_lowercase().as_str());
  }

  /// Clears all headers.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.set("Content-Type", "text/plain");
  /// headers.set("Accept", "application/json");
  /// headers.clear();
  ///
  /// assert_eq!(headers.get("Content-Type"), None);
  /// assert_eq!(headers.get("Accept"), None);
  /// ```
  pub fn clear(&mut self) {
    self.0.clear();
  }

  /// Returns the number of headers.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::Headers;
  ///
  /// let mut headers = Headers::new();
  /// headers.set("Content-Type", "text/plain");
  /// headers.set("Accept", "application/json");
  ///
  /// assert_eq!(headers.len(), 2);
  /// ```
  pub fn len(&self) -> usize {
    self.0.len()
  }

  /// Checks if the headers are empty.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::Headers;
  ///
  /// let headers = Headers::new();
  ///
  /// assert_eq!(headers.is_empty(), true);
  /// ```
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Returns an iterator over the headers.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::{Headers, Header};
  /// let mut headers = Headers::new();
  /// headers.add("Accept", "text/plain");
  /// headers.add("Accept", "application/json");
  ///
  /// for (key, values) in headers.iter() {
  ///    println!("{}: {:?}", key, values);
  /// }
  ///
  /// # assert_eq!(headers.iter().collect::<Vec<(&String, &Header)>>(), vec![
  /// #   (&"accept".to_string(), &Header::Multiple(vec![
  /// #     "text/plain".to_string(),
  /// #     "application/json".to_string()
  /// #   ]))
  /// # ]);
  /// ```
  pub fn iter(&self) -> impl Iterator<Item = (&String, &Header)> {
    self.0.iter()
  }
}

impl Default for Headers {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(feature = "napi")]
mod napi_headers {
  use super::*;

  use std::ptr;

  use napi::bindgen_prelude::*;
  use napi::sys;

  impl FromNapiValue for Headers {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> Result<Self> {
      let mut header_type = sys::ValueType::napi_undefined;
      unsafe { check_status!(sys::napi_typeof(env, value, &mut header_type)) }?;

      let header_type: ValueType = header_type.into();

      if header_type != ValueType::Object {
        return Err(napi::Error::new(
          napi::Status::InvalidArg,
          "Expected an object for Headers",
        ));
      }

      let mut headers = Headers::new();
      unsafe {
        let mut keys: sys::napi_value = ptr::null_mut();
        check_status!(
          sys::napi_get_property_names(env, value, &mut keys),
          "Failed to get Headers property names"
        )?;

        let mut key_count = 0;
        check_status!(sys::napi_get_array_length(env, keys, &mut key_count))?;

        for i in 0..key_count {
          let mut key: sys::napi_value = ptr::null_mut();
          check_status!(
            sys::napi_get_element(env, keys, i, &mut key),
            "Failed to get header name"
          )?;
          let key_str = String::from_napi_value(env, key)?;
          let key_cstr = std::ffi::CString::new(key_str.clone())?;

          let mut header_value: sys::napi_value = ptr::null_mut();
          check_status!(
            sys::napi_get_named_property(env, value, key_cstr.as_ptr(), &mut header_value),
            "Failed to get header value"
          )?;

          if let Ok(header) = Header::from_napi_value(env, header_value) {
            headers.set(key_str, header);
          }
        }
      }

      Ok(headers)
    }
  }

  impl ToNapiValue for Headers {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> Result<sys::napi_value> {
      let mut result: sys::napi_value = ptr::null_mut();
      unsafe {
        check_status!(
          sys::napi_create_object(env, &mut result),
          "Failed to create Headers object"
        )?;

        for (key, header) in value.iter() {
          let key_cstr = std::ffi::CString::new(key.to_string())?;
          let value_napi_value = Header::to_napi_value(env, header.to_owned())?;

          check_status!(
            sys::napi_set_named_property(env, result, key_cstr.as_ptr(), value_napi_value),
            "Failed to set header property"
          )?;
        }
      }

      Ok(result)
    }
  }
}

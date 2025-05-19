use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug, Clone)]
pub enum Header {
  Single(String),
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

/// A multi-map of HTTP headers.
///
/// # Examples
///
/// ```
/// # use lang_handler::Headers;
/// let mut headers = Headers::new();
/// headers.set("Content-Type", "text/plain");
///
/// assert_eq!(headers.get("Content-Type"), Some(&vec!["text/plain".to_string()]));
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
    self.0.contains_key(key.as_ref()/*.to_lowercase().as_str()*/)
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
  /// assert_eq!(headers.get("Accept"), Some(&"application/json".to_string()));
  /// ```
  pub fn get<K>(&self, key: K) -> Option<String>
  where
    K: AsRef<str>,
  {
    match self.0.get(key.as_ref()/*.to_lowercase().as_str()*/) {
      Some(Header::Single(value)) => Some(value.clone()),
      Some(Header::Multiple(values)) => values.last().cloned(),
      None => None,
    }
  }

  /// Returns all values associated with a header field as a Vec<String>.
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
    match self.0.get(key.as_ref()/*.to_lowercase().as_str()*/) {
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
    let result = self.get_all(key).join(",");
    if result.is_empty() {
      None
    } else {
      Some(result)
    }
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
  /// assert_eq!(headers.get("Content-Type"), Some(&"text/html".to_string()));
  /// ```
  pub fn set<K, V>(&mut self, key: K, value: V)
  where
    K: Into<String>,
    V: Into<String>,
  {
    self
      .0
      .insert(key.into()/*.to_lowercase()*/, Header::Single(value.into()));
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
  /// assert_eq!(headers.get("Accept"), Some(&vec![
  ///   "text/plain".to_string(),
  ///   "application/json".to_string()
  /// ]));
  /// ```
  pub fn add<K, V>(&mut self, key: K, value: V)
  where
    K: Into<String>,
    V: Into<String>,
  {
    let key = key.into()/*.to_lowercase()*/;
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
    self.0.remove(key.as_ref()/*.to_lowercase().as_str()*/);
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
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.set("Content-Type", "text/plain");
  /// headers.set("Accept", "application/json");
  ///
  /// assert_eq!(headers.len(), 2);
  /// ```
  pub fn len(&self) -> usize {
    self.0.len()
  }

  /// Returns an iterator over the headers.
  ///
  /// # Examples
  ///
  /// ```
  /// # use lang_handler::Headers;
  /// let mut headers = Headers::new();
  /// headers.set("Accept", "text/plain");
  /// headers.set("Accept", "application/json");
  ///
  /// for (key, values) in headers.iter() {
  ///    println!("{}: {:?}", key, values);
  /// }
  /// ```
  pub fn iter(&self) -> impl Iterator<Item = (&String, &Header)> {
    self.0.iter()
  }
}

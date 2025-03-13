use std::collections::HashMap;

/// A multi-map of HTTP headers.
///
/// # Examples
///
/// ```
/// use lang_handler::Headers;
///
/// let mut headers = Headers::new();
/// headers.set("Content-Type", "text/plain");
/// assert_eq!(headers.get("Content-Type"), Some(&vec!["text/plain".to_string()]));
/// ```
#[derive(Debug, Clone)]
pub struct Headers(HashMap<String, Vec<String>>);

impl Headers {
    /// Creates a new `Headers` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use lang_handler::Headers;
    ///
    /// let headers = Headers::new();
    /// ```
    pub fn new() -> Self {
        Headers(HashMap::new())
    }

    /// Returns the values associated with a header field.
    ///
    /// # Examples
    ///
    /// ```
    /// use lang_handler::Headers;
    ///
    /// let mut headers = Headers::new();
    /// headers.set("Accept", "text/plain");
    /// headers.set("Accept", "application/json");
    ///
    /// assert_eq!(headers.get("Accept"), Some(&vec![
    ///   "text/plain".to_string(),
    ///   "application/json".to_string()
    /// ]));
    /// ```
    pub fn get<K>(&self, key: K) -> Option<&Vec<String>>
    where
        K: AsRef<str>,
    {
        self.0.get(key.as_ref())
    }

    /// Sets a header field with the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use lang_handler::Headers;
    ///
    /// let mut headers = Headers::new();
    /// headers.set("Content-Type", "text/plain");
    /// assert_eq!(headers.get("Content-Type"), Some(&vec!["text/plain".to_string()]));
    /// ```
    pub fn set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.0
            .entry(key.into())
            .or_insert_with(Vec::new)
            .push(value.into());
    }

    /// Removes a header field.
    ///
    /// # Examples
    ///
    /// ```
    /// use lang_handler::Headers;
    ///
    /// let mut headers = Headers::new();
    /// headers.set("Content-Type", "text/plain");
    /// headers.remove("Content-Type");
    /// assert_eq!(headers.get("Content-Type"), None);
    /// ```
    pub fn remove<K>(&mut self, key: K)
    where
        K: AsRef<str>,
    {
        self.0.remove(key.as_ref());
    }

    /// Returns an iterator over the headers.
    ///
    /// # Examples
    ///
    /// ```
    /// use lang_handler::Headers;
    ///
    /// let mut headers = Headers::new();
    /// headers.set("Accept", "text/plain");
    /// headers.set("Accept", "application/json");
    ///
    /// for (key, values) in headers.iter() {
    ///    println!("{}: {:?}", key, values);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.0.iter()
    }

    /// Returns an iterator over the header values.
    ///
    /// # Examples
    ///
    /// ```
    /// use lang_handler::Headers;
    ///
    /// let mut headers = Headers::new();
    /// headers.set("Accept", "text/plain");
    /// headers.set("Accept", "application/json");
    ///
    /// for value in headers.iter_values() {
    ///    println!("{:?}", value);
    /// }
    /// ```
    pub fn iter_values(&self) -> impl Iterator<Item = &String> {
        self.0.values().flatten()
    }
}

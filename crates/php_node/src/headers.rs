use std::ptr;

use napi::Result;
use napi::{sys, sys::{napi_env, napi_value}};
use napi::bindgen_prelude::*;

use php::{Headers, Header};

pub struct Entry<K, V>(K, V);

// This represents a map entries key/value pair.
impl<T1, T2> ToNapiValue for Entry<T1, T2>
where
    T1: ToNapiValue,
    T2: ToNapiValue,
{
    unsafe fn to_napi_value(env: napi_env, val: Self) -> Result<napi_value> {
        let Entry(key, value) = val;
        let key_napi_value = T1::to_napi_value(env, key)?;
        let value_napi_value = T2::to_napi_value(env, value)?;

        let mut result: napi_value = ptr::null_mut();
        unsafe {
            check_status!(
                sys::napi_create_array_with_length(env, 2, &mut result),
                "Failed to create entry key/value pair"
            )?;

            check_status!(
                sys::napi_set_element(env, result, 0, key_napi_value),
                "Failed to set entry key"
            )?;

            check_status!(
                sys::napi_set_element(env, result, 1, value_napi_value),
                "Failed to set entry value"
            )?;
        };

        Ok(result)
    }
}

/// A multi-map of HTTP headers.
///
/// # Examples
///
/// ```js
/// const headers = new Headers();
/// headers.set('Content-Type', 'application/json');
/// const contentType = headers.get('Content-Type');
/// ```
#[napi(js_name = "Headers")]
pub struct PhpHeaders {
    headers: Headers
}

impl PhpHeaders {
    // Create a new PHP headers instance.
    pub fn new(headers: Headers) -> Self {
        PhpHeaders {
            headers
        }
    }
}

#[napi]
impl PhpHeaders {
    /// Create a new PHP headers instance.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// ```
    #[napi(constructor)]
    pub fn constructor() -> Self {
        PhpHeaders {
            headers: Headers::new()
        }
    }

    /// Get the last set value for a given header key.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Accept', 'application/json');
    /// headers.set('Accept', 'text/html');
    ///
    /// console.log(headers.get('Accept')); // text/html
    /// ```
    #[napi]
    pub fn get(&self, key: String) -> Option<String> {
        self.headers.get(&key)
    }

    /// Get all values for a given header key.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Accept', 'application/json');
    /// headers.set('Accept', 'text/html');
    ///
    /// for (const mime of headers.getAll('Accept')) {
    ///   console.log(mime);
    /// }
    /// ```
    #[napi]
    pub fn get_all(&self, key: String) -> Vec<String> {
        self.headers.get_all(&key)
    }

    /// Get all values for a given header key as a comma-separated string.
    ///
    /// This is useful for headers that can have multiple values, such as `Accept`.
    /// But note that some headers like `Set-Cookie`, expect separate lines.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Accept', 'application/json');
    /// headers.set('Accept', 'text/html');
    ///
    /// console.log(headers.getLine('Accept')); // application/json, text/html
    /// ```
    #[napi]
    pub fn get_line(&self, key: String) -> Option<String> {
        self.headers.get_line(&key)
    }

    /// Check if a header key exists.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    ///
    /// console.log(headers.has('Content-Type')); // true
    /// console.log(headers.has('Accept')); // false
    /// ```
    #[napi]
    pub fn has(&self, key: String) -> bool {
        self.headers.has(&key)
    }

    /// Set a header key/value pair.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// ```
    #[napi]
    pub fn set(&mut self, key: String, value: String) {
        self.headers.set(key, value)
    }

    /// Add a value to a header key.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Accept', 'application/json');
    /// headers.add('Accept', 'text/html');
    ///
    /// console.log(headers.get('Accept')); // application/json, text/html
    /// ```
    #[napi]
    pub fn add(&mut self, key: String, value: String) {
        self.headers.add(key, value)
    }

    /// Delete a header key/value pair.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.delete('Content-Type');
    /// ```
    #[napi]
    pub fn delete(&mut self, key: String) {
        self.headers.remove(&key)
    }

    /// Clear all header entries.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.set('Accept', 'application/json');
    /// headers.clear();
    ///
    /// console.log(headers.has('Content-Type')); // false
    /// console.log(headers.has('Accept')); // false
    /// ```
    #[napi]
    pub fn clear(&mut self) {
        self.headers.clear()
    }

    /// Get the number of header entries.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.set('Accept', 'application/json');
    ///
    /// console.log(headers.size); // 2
    /// ```
    #[napi(getter)]
    pub fn size(&self) -> u32 {
        self.headers.len() as u32
    }

    /// Get an iterator over the header entries.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.set('Accept', 'application/json');
    ///
    /// for (const [key, values] of headers.entries()) {
    ///   console.log(`${key}: ${values.join(', ')}`);
    /// }
    /// ```
    #[napi]
    pub fn entries(&self) -> Vec<Entry<String, Vec<String>>> {
        self.headers.iter().map(|(k, v)| {
            Entry(k.to_owned(), match v {
                Header::Single(value) => vec![value.clone()],
                Header::Multiple(vec) => vec.clone(),
            })
        }).collect()
    }

    /// Get an iterator over the header keys.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.set('Accept', 'application/json');
    ///
    /// for (const key of headers.keys()) {
    ///   console.log(key);
    /// }
    /// ```
    #[napi]
    pub fn keys(&self) -> Vec<String> {
        self.headers.iter().map(|(k, _)| k.to_owned()).collect()
    }

    /// Get an iterator over the header values.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.set('Accept', 'application/json');
    ///
    /// for (const value of headers.values()) {
    ///   console.log(value);
    /// }
    /// ```
    #[napi]
    pub fn values(&self) -> Vec<String> {
        self.entries().into_iter().flat_map(|entry| entry.1).collect()
    }

    /// Execute a callback for each header entry.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.set('Accept', 'application/json');
    ///
    /// headers.forEach(([key, values]) => {
    ///   console.log(`${key}: ${values.join(', ')}`);
    /// });
    /// ```
    #[napi]
    pub fn for_each<F: Fn(Vec<String>, String, &This) -> Result<()>>(&self, this: This, callback: F) -> Result<()> {
        for entry in self.entries() {
            let entry = Entry(entry.0, entry.1);
            callback(entry.1, entry.0, &this)?;
        }
        Ok(())
    }
}

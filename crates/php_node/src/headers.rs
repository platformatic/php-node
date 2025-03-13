use std::ptr;

use napi::Result;
use napi::{sys, sys::{napi_env, napi_value}};
use napi::bindgen_prelude::*;

use php::Headers;

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

    /// Get the values for a given header key.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Accept', 'application/json');
    /// headers.set('Accept', 'text/html');
    ///
    /// for (const mime of headers.get('Accept')) {
    ///   console.log(mime);
    /// }
    /// ```
    #[napi]
    pub fn get(&self, key: String) -> Option<Vec<String>> {
        self.headers.get(&key).map(|v| v.to_owned())
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

    /// Remove a header key/value pair.
    ///
    /// # Examples
    ///
    /// ```js
    /// const headers = new Headers();
    /// headers.set('Content-Type', 'application/json');
    /// headers.remove('Content-Type');
    /// ```
    #[napi]
    pub fn remove(&mut self, key: String) {
        self.headers.remove(&key)
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
        self.headers.iter().map(|(k, v)| Entry(k.to_owned(), v.to_owned())).collect()
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
        self.headers.iter_values().map(|v| v.to_owned()).collect()
    }
}

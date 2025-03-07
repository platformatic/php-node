#[macro_use]
extern crate napi_derive;

use napi::{Error, Task, Env, Result};
use napi::bindgen_prelude::*;

use php::{Embed, Handler, Headers, Request, Response, RequestBuilder};

pub struct Entry<K, V>(K, V);

// This represents a map entries key/value pair.
impl<T1, T2> ToNapiValue for Entry<T1, T2>
where
    T1: ToNapiValue,
    T2: ToNapiValue,
{
    unsafe fn to_napi_value(env: napi::sys::napi_env, val: Self) -> napi::Result<napi::sys::napi_value> {
        let Entry(key, value) = val;
        let key_napi_value = T1::to_napi_value(env, key)?;
        let value_napi_value = T2::to_napi_value(env, value)?;

        let mut result: napi::sys::napi_value = std::ptr::null_mut();
        unsafe {
            check_status!(
                napi::sys::napi_create_array_with_length(env, 2, &mut result),
                "Failed to create entry key/value pair"
            )?;

            check_status!(
                napi::sys::napi_set_element(env, result, 0, key_napi_value),
                "Failed to set entry key"
            )?;

            check_status!(
                napi::sys::napi_set_element(env, result, 1, value_napi_value),
                "Failed to set entry value"
            )?;
        };

        Ok(result)
    }
}

#[napi(js_name = "Headers")]
pub struct PhpHeaders {
    headers: Headers
}

#[napi]
impl PhpHeaders {
    #[napi]
    pub fn get(&self, key: String) -> Option<Vec<String>> {
        self.headers.get(&key).map(|v| v.to_owned())
    }

    #[napi]
    pub fn set(&mut self, key: String, value: String) {
        self.headers.set(key, value)
    }

    #[napi]
    pub fn remove(&mut self, key: String) {
        self.headers.remove(&key)
    }

    #[napi]
    pub fn entries(&self) -> Vec<Entry<String, Vec<String>>> {
        self.headers.iter().map(|(k, v)| Entry(k.to_owned(), v.to_owned())).collect()
    }

    #[napi]
    pub fn keys(&self) -> Vec<String> {
        self.headers.iter().map(|(k, _)| k.to_owned()).collect()
    }

    #[napi]
    pub fn values(&self) -> Vec<String> {
        self.headers.iter_values().map(|v| v.to_owned()).collect()
    }
}

#[napi(object)]
#[derive(Default)]
pub struct PhpRequestOptions {
  pub method: Option<String>,
  pub url: Option<String>,
  pub headers: Option<Object>,
  pub body: Option<String>
}

#[napi(js_name = "Request")]
pub struct PhpRequest {
    request: Request
}

#[napi]
impl PhpRequest {
    #[napi(constructor)]
    pub fn new(options: Option<PhpRequestOptions>) -> Self {
        let opts = options.unwrap_or_default();
        let mut builder: RequestBuilder = Request::builder();

        if let Some(method) = opts.method {
            builder = builder.method(method)
        }

        if let Some(url) = opts.url {
            builder = builder.url(url).expect("invalid url")
        }

        if let Some(headers) = opts.headers {
            for key in Object::keys(&headers).unwrap() {
                let values: Vec<String> = headers.get(&key).unwrap().unwrap();
                for value in values {
                    builder = builder.header(key.clone(), value.clone())
                }
            }
        }

        if let Some(body) = opts.body {
            builder = builder.body(body)
        }

        PhpRequest {
            request: builder.build()
        }
    }

    #[napi(getter, enumerable = true)]
    pub fn method(&self) -> String {
        self.request.method().to_owned()
    }

    #[napi(getter, enumerable = true)]
    pub fn url(&self) -> String {
        self.request
            .url()
            .as_str()
            .to_owned()
    }

    #[napi(getter, enumerable = true)]
    pub fn headers(&self) -> PhpHeaders {
        PhpHeaders {
            headers: self.request.headers().clone()
        }
    }

    #[napi(getter, enumerable = true)]
    pub fn body(&self) -> String {
        self.request
            .body()
            .map(|v| v.to_owned())
            .unwrap_or_default()
    }
}

impl PhpRequest {
    fn to_inner(&self) -> Request {
        self.request.clone()
    }
}

#[napi(object)]
#[derive(Clone, Default)]
pub struct PhpOptions {
    pub code: String,
    pub file: Option<String>
}

#[napi]
pub struct Php {
    pub options: PhpOptions
}

#[napi]
impl Php {
    #[napi(constructor)]
    pub fn new(options: PhpOptions) -> Self {
        Php { options }
    }

    #[napi]
    pub fn handle_request(&self, request: &PhpRequest) -> AsyncTask<PhpRequestTask> {
        AsyncTask::new(PhpRequestTask {
            options: self.options.clone(),
            request: request.to_inner()
        })
    }
}

pub struct PhpRequestTask {
    options: PhpOptions,
    request: Request
}

impl Task for PhpRequestTask {
    type Output = Response;
    type JsValue = PhpResponse;

    fn compute(&mut self) -> Result<Self::Output> {
        let code = self.options.code.clone();
        let filename = self.options.file.clone();
        // TODO: Need to figure out how to send an Embed across threads
        // so we can reuse the same Embed instance for multiple requests.
        let embed = Embed::new(code, filename);

        embed
            .handle(self.request.clone())
            .map_err(|err| Error::from_reason(err))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(PhpResponse {
            response: output
        })
    }
}

#[napi]
pub struct PhpResponse {
    response: Response
}

#[napi]
impl PhpResponse {
    #[napi(getter, enumerable = true)]
    pub fn status(&self) -> u32 {
        self.response.status() as u32
    }

    #[napi(getter, enumerable = true)]
    pub fn headers(&self) -> PhpHeaders {
        PhpHeaders {
            headers: self.response.headers().clone()
        }
    }

    #[napi(getter, enumerable = true)]
    pub fn body(&self) -> String {
        self.response.body().to_owned()
    }
}

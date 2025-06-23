use std::env;

#[cfg(feature = "napi-support")]
extern crate napi_build;

fn main() {
  #[cfg(feature = "napi-support")]
  napi_build::setup();

  // Check for manual PHP_RPATH override, otherwise try LD_PRELOAD_PATH,
  // and finally fallback to hard-coded /usr/local/lib path.
  //
  // PHP_RPATH may also be $ORIGIN to instruct the build to search for libphp
  // in the same directory as the *.node bindings file.
  let php_rpath = env::var("PHP_RPATH")
    .or_else(|_| env::var("LD_PRELOAD_PATH"))
    .unwrap_or("/usr/local/lib".to_string());

  println!("cargo:rustc-link-search={php_rpath}");
  println!("cargo:rustc-link-lib=dylib=php");
  println!("cargo:rustc-link-arg=-Wl,-rpath,{php_rpath}");
}

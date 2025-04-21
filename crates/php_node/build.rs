extern crate napi_build;

fn main() {
  napi_build::setup();
  println!("cargo:rustc-link-lib=dylib=php");
  println!("cargo:rustc-link-arg=-Wl,-rpath=$ORIGIN");
}

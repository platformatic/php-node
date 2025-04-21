extern crate cbindgen;

use std::env;
use std::path::PathBuf;

fn main() {
  let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap())
    .join("../../..")
    .canonicalize()
    .unwrap();

  let header_path = out_dir.join("lang_handler.h");

  cbindgen::Builder::new()
    .with_crate(crate_dir)
    .with_include_guard("LANG_HANDLER_H")
    .with_language(cbindgen::Language::C)
    .generate()
    .expect("Unable to generate bindings")
    .write_to_file(header_path);
}

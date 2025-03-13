extern crate cbindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();

    let target_str = format!("../../target/{}", profile);
    let header_path = PathBuf::from(crate_dir.clone())
        .join(target_str)
        .join("lang_handler.h");

    cbindgen::Builder::new()
      .with_crate(crate_dir)
      .with_include_guard("LANG_HANDLER_H")
      .with_language(cbindgen::Language::C)
      .generate()
      .expect("Unable to generate bindings")
      .write_to_file(header_path);
}

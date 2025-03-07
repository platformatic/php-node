extern crate cbindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = PathBuf::from(crate_dir.clone());

    cbindgen::Builder::new()
      .with_crate(crate_dir)
      .with_include_guard("LANG_HANDLER_H")
      .with_language(cbindgen::Language::C)
      .with_parse_deps(true)
      .with_parse_include(&["url"])
      .include_item("Url")
      // .rename_item("Request", "lh_request_t")
      // .rename_item("RequestBuilder", "lh_request_builder_t")
      // .rename_item("Response", "lh_response_t")
      // .rename_item("ResponseBuilder", "lh_response_builder_t")
      // .rename_item("Headers", "lh_headers_t")
      // .rename_item("Url", "lh_url_t")
      .generate()
      .expect("Unable to generate bindings")
      .write_to_file(out_dir.join("../../target/release/lang_handler.h"));
}

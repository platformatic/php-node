use std::{env::current_dir, fs::File, io::Write, path::PathBuf};

use php::{
  rewrite::{PathRewriter, Rewriter},
  Embed, Handler, Request,
};

pub fn main() {
  let _temp_file = TempFile::new("index.php", "<?php echo \"Hello, world!\" ?>");

  let docroot = current_dir().expect("should have current_dir");

  let rewriter = PathRewriter::new("test", "index").expect("should be valid regex");

  let maybe_rewriter: Option<Box<dyn Rewriter>> = Some(rewriter);
  let embed = Embed::new_with_args(docroot, maybe_rewriter, std::env::args())
    .expect("should construct embed");

  let request = Request::builder()
    .method("POST")
    .url("http://example.com/test.php")
    .header("Content-Type", "text/html")
    .header("Content-Length", 13.to_string())
    .body("Hello, World!")
    .build()
    .expect("should build request");

  println!("request: {:#?}", request);

  let response = embed
    .handle(request.clone())
    .expect("should handle request");

  println!("response: {:#?}", response);
}

struct TempFile(PathBuf);

impl TempFile {
  pub fn new<P, S>(path: P, contents: S) -> Self
  where
    P: Into<PathBuf>,
    S: Into<String>,
  {
    let path = path.into();
    let mut file = File::create(path.clone()).unwrap();
    file.write_all(contents.into().as_bytes()).unwrap();
    Self(path)
  }
}

impl Drop for TempFile {
  fn drop(&mut self) {
    std::fs::remove_file(&self.0).unwrap();
  }
}

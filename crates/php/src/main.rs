use std::{env::current_dir, fs::File, io::Write, path::PathBuf};

use php::{Embed, Handler};
use bytes::BytesMut;

#[tokio::main]
pub async fn main() {
  let _temp_file = TempFile::new("index.php", "<?php echo \"Hello, world!\" ?>");

  let docroot = current_dir().expect("should have current_dir");

  let embed = Embed::new_with_args(docroot, None, std::env::args())
    .expect("should construct embed");

  let request = http_handler::request::Request::builder()
    .method("POST")
    .uri("/test.php")
    .header("Content-Type", "text/html")
    .header("Content-Length", "13")
    .body(BytesMut::from("Hello, World!"))
    .expect("should build request");

  println!("request: {:#?}", request);

  let response = embed
    .handle(request.clone())
    .await
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

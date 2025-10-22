use std::{env::current_dir, fs::File, io::Write, path::PathBuf};

use bytes::BytesMut;
use php_node::{rewrite::PathRewriter, Embed, Handler, Request, RequestRewriter};

#[tokio::main]
async fn main() {
  let _temp_file = TempFile::new("index.php", "<?php echo \"Hello, world!\"; ?>");

  let docroot = current_dir().expect("should have current_dir");

  let rewriter = PathRewriter::new("test", "index").expect("should be valid regex");

  let maybe_rewriter: Option<Box<dyn RequestRewriter>> = Some(Box::new(rewriter));
  let embed = Embed::new_with_args(docroot, maybe_rewriter, std::env::args())
    .expect("should construct embed");

  // Build request using the re-exported Request type from http crate
  let mut request = Request::new(BytesMut::from("Hello, World!"));
  *request.method_mut() = "POST".parse().unwrap();
  *request.uri_mut() = "http://example.com/test.php".parse().unwrap();
  request
    .headers_mut()
    .insert("Content-Type", "text/html".parse().unwrap());
  request
    .headers_mut()
    .insert("Content-Length", "13".parse().unwrap());

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

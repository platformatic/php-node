use php::{Embed, Handler, Request};

pub fn main() {
  let docroot = std::env::current_dir().expect("should have current_dir");

  let embed = Embed::new_with_args(docroot, std::env::args()).expect("should construct embed");

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

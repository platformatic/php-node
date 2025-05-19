use php::{Embed, Handler, Request};

pub fn main() {
  let docroot = std::env::current_dir()
    .expect("should have current_dir");
  let embed = Embed::new_with_args(docroot, std::env::args());

  let request = Request::builder()
    .method("POST")
    .url("http://example.com/test.php")
    .expect("invalid url")
    .header("Content-Type", "text/html")
    .header("Content-Length", 13.to_string())
    .body("Hello, World!")
    .build();

  println!("request: {:#?}", request);

  let response = embed
    .handle(request.clone())
    .expect("failed to handle request");

  println!("response: {:#?}", response);
}

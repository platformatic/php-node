use php::{Embed, Request, Handler};

pub fn main() {
    let code = "
        http_response_code(123);
        header('Content-Type: text/plain');
        echo file_get_contents(\"php://input\");
    ";
    let filename = Some("test.php");
    let embed = Embed::new_with_args(code, filename, std::env::args());

    let request = Request::builder()
        .method("POST")
        .url("http://example.com/test.php").expect("invalid url")
        .header("Content-Type", "text/html")
        .header("Content-Length", 13.to_string())
        .body("Hello, World!")
        .build();

    println!("request: {:#?}", request);

    let response = embed.handle(request.clone())
        .expect("failed to handle request");

    println!("response: {:#?}", response);
}

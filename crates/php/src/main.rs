use php::{Embed, Request, Handler};

pub fn main() {
    let code = "
        http_response_code(404);
        header('Content-Type: text/html');
        print('hello');
        flush();
    ";
    let filename = Some("test.php");
    let embed = Embed::new_with_args(code, filename, std::env::args());
    // let embed = Embed::new(code, filename);

    let request = Request::builder()
        .method("POST")
        .url("http://example.com/test.php").expect("invalid url")
        .header("Content-Type", "text/html")
        .header("Content-Length", 13.to_string())
        .body("Hello, World!")
        .build();

    println!("=== request ===");
    println!("method: {}", request.method());
    println!("url: {:?}", request.url());
    println!("headers: {:?}", request.headers());
    println!("body: {:?}", request.body());
    println!("");

    let response = embed.handle(request.clone()).unwrap();

    println!("\n=== response ===");
    println!("status: {:?}", response.status());
    println!("headers: {:?}", response.headers());
    println!("body: {:?}", response.body());
    println!("");
}

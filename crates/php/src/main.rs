use php::{Embed, Request};

pub fn main() {
    let embed = Embed::new_with_args(std::env::args());

    let request = Request::builder()
        .method("GET")
        .path("/test.php")
        .body("Hello, World!")
        .build();

    println!("method: {}", request.method());
    println!("path: {}", request.path());
    println!("body: {}", request.body());

    let response = embed.handle_request(
        ";echo 'Hello, World!';",
        Some("test.php"),
        request.clone()
    );

    println!("Request: {:?}", request);
    println!("Response: {:?}", response);
}

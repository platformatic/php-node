#[macro_use]
extern crate napi_derive;

use php::{Embed, Request};

#[napi]
pub fn handle_request() {
    let embed = Embed::new();

    let request = Request::builder()
        .method("GET")
        .path("/test.php")
        .body("Hello, World!")
        .build();

    println!("=== request ===");
    println!("method: {}", request.method());
    println!("path: {}", request.path());
    println!("body: {}", request.body());
    println!("");

    let response = embed.handle_request(
        "
            echo 'Hello, World!\n';
            http_response_code(400);
            echo $_SERVER['REQUEST_METHOD'] . '\n';
        ",
        Some("test.php"),
        request.clone()
    );

    println!("\n=== response ===");
    println!("status: {:?}", response.status());
    println!("body: {:?}", response.body());
}

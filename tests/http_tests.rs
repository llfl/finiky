use finiky::http::server::HttpServer;

#[test]
fn test_content_type_guessing() {
    assert_eq!(HttpServer::guess_content_type("test.html"), "text/html");
    assert_eq!(HttpServer::guess_content_type("test.HTML"), "text/html");
    assert_eq!(HttpServer::guess_content_type("test.txt"), "text/plain");
    assert_eq!(
        HttpServer::guess_content_type("boot.efi"),
        "application/octet-stream"
    );
    assert_eq!(HttpServer::guess_content_type("image.png"), "image/png");
    assert_eq!(HttpServer::guess_content_type("image.jpg"), "image/jpeg");
    assert_eq!(HttpServer::guess_content_type("style.css"), "text/css");
    assert_eq!(
        HttpServer::guess_content_type("script.js"),
        "application/javascript"
    );
    assert_eq!(
        HttpServer::guess_content_type("data.json"),
        "application/json"
    );
    assert_eq!(
        HttpServer::guess_content_type("boot.0"),
        "application/octet-stream"
    );
    assert_eq!(
        HttpServer::guess_content_type("unknown"),
        "application/octet-stream"
    );
}

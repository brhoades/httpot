use std::sync::Arc;

use tokio::net::TcpStream;

use httpot::{
    fs,
    honeypot::php,
    http::{
        request::{Method, Request},
        response::{Response, ResponseBuilder, StatusCode},
        stock_responses::*,
    },
    prelude::*,
};

pub fn router(conn: TcpStream, r: &Request) -> Result<Response> {
    // invalid methods
    match r.method {
        Method::GET => (),
        Method::OPTIONS => (),
        _ => {
            return Ok(generic_status(conn, StatusCode::MethodNotAllowed)
                .add_headers("Allow", vec!["GET", "OPTIONS"])
                .build()?)
        }
    };

    if php::is_easter_egg(r) {
        return php::easter_egg(conn, r);
    }

    match r.url.path() {
        "/hello" => Ok(hello_world(conn)),
        "/favicon.ico" => Ok(not_found(conn)),
        path if path.ends_with("/") => fake_directory_tree(conn, r),
        _ => Ok(not_found(conn)),
    }
}

const SEED: &str = "seedv1";

pub fn fake_directory_tree(conn: TcpStream, req: &Request) -> Result<Response> {
    let body = fs::fake::gen_fake_listing(SEED, req.url.path());

    Ok(ResponseBuilder::ok(Arc::new(conn))
        .body(body)
        .add_header("Content-Type", "text/html")
        .build()?)
}

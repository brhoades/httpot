use httpot::{
    http::{
        request::{Method, Request},
        response::{Response, StatusCode},
    },
    prelude::*,
};

pub async fn router(r: &Request) -> Result<Response> {
    use crate::stock_responses::*;

    // invalid methods
    match r.method {
        Method::GET => (),
        Method::OPTIONS => (),
        _ => {
            return Ok(generic_status(StatusCode::MethodNotAllowed)
                .add_headers("Allow", vec!["GET", "OPTIONS"])
                .build()?)
        }
    };

    match r.url.path() {
        "/hello" => Ok(hello_world()),
        "/favicon.ico" => Ok(not_found()),
        path if path.ends_with("/") => fake_directory_tree(r),
        _ => Ok(not_found()),
    }
}

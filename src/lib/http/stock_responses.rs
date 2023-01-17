use typed_html::{dom::DOMTree, html, text, types::Metadata};

use crate::{
    fs::fake,
    http::{
        request::Request,
        response::{Response, ResponseBuilder, StatusCode},
    },
    prelude::*,
};

#[macro_export]
macro_rules! boilerplate {
    ($title:expr, $tokens:expr) => {
      html!(
        <html>
          <head>
            <title>{text!("{}", $title)}</title>
            <meta name=Metadata::Description content="httpot" />
          </head>
          <body>
            { $tokens }
          </body>
        </html>
      )
    };
    ($tokens:expr) => {
        boilerplate!("", text!(""), $tokens)
    };
}

pub fn hello_world() -> Response {
    let body: DOMTree<String> = boilerplate!("Hello World!", html!(<h1>"Hello, World!"</h1>));

    ResponseBuilder::ok()
        .add_header("Content-Type", "text/html")
        .body(body.to_string())
        .build()
        .unwrap()
}

pub fn not_found() -> Response {
    let body: DOMTree<String> = boilerplate!("Not Found", html!(<h1>"Not Found"</h1>));

    ResponseBuilder::not_found()
        .add_header("Content-Type", "text/html")
        .body(body.to_string())
        .build()
        .unwrap()
}
pub fn generic_status(status: StatusCode) -> ResponseBuilder {
    let stat_str = text!("{}", status.to_string());
    let body: DOMTree<String> = boilerplate!(stat_str, html!(<h1>{stat_str}</h1>));

    let mut resp = ResponseBuilder::default();
    resp.add_header("Content-Type", "text/html")
        .body(body.to_string())
        .status_code(status);
    resp
}

const SEED: &str = "seedv1";

pub fn fake_directory_tree(req: &Request) -> Result<Response> {
    //
    Ok(fake::gen_fake_listing(SEED, req.url.path()))
}

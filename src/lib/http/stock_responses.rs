use std::sync::Arc;

use tokio::net::TcpStream;
use typed_html::{dom::DOMTree, html, text, types::Metadata};

use crate::http::response::{Response, ResponseBuilder, StatusCode};

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

pub fn hello_world(out: TcpStream) -> Response {
    let body: DOMTree<String> = boilerplate!("Hello World!", html!(<h1>"Hello, World!"</h1>));

    ResponseBuilder::ok(Arc::new(out))
        .add_header("Content-Type", "text/html")
        .body(body.to_string())
        .build()
        .unwrap()
}

pub fn not_found(out: TcpStream) -> Response {
    let body: DOMTree<String> = boilerplate!("Not Found", html!(<h1>"Not Found"</h1>));

    ResponseBuilder::not_found(Arc::new(out))
        .add_header("Content-Type", "text/html")
        .body(body.to_string())
        .build()
        .unwrap()
}
pub fn generic_status(out: TcpStream, status: StatusCode) -> ResponseBuilder {
    let stat_str = text!("{}", status.to_string());
    let body: DOMTree<String> = boilerplate!(stat_str, html!(<h1>{stat_str}</h1>));

    let mut resp = ResponseBuilder::default(Arc::new(out));
    resp.add_header("Content-Type", "text/html")
        .body(body.to_string())
        .status_code(status);
    resp
}

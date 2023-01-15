use typed_html::{dom::DOMTree, html, text, types::Metadata};

use httpot::{
    fs::fake,
    http::{
        request::Request,
        response::{Response, ResponseBuilder},
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

pub(crate) fn hello_world() -> Response {
    let body: DOMTree<String> = boilerplate!("Hello World!", html!(<h1>"Hello, World!"</h1>));

    ResponseBuilder::ok()
        .add_header("Server", "Apache/2.2.14 (Win32)")
        .add_header("Content-Type", "text/html")
        .body(body.to_string())
        .build()
        .unwrap()
}

pub(crate) fn not_found() -> Response {
    let body: DOMTree<String> = boilerplate!("Not Found", html!(<h1>"Not Found"</h1>));

    ResponseBuilder::not_found()
        .add_header("Server", "Apache/2.2.14 (Win32)")
        .body(body.to_string())
        .build()
        .unwrap()
}

const SEED: &str = "seedv1";

pub(crate) fn fake_directory_tree(req: &Request) -> Result<Response> {
    Ok(fake::gen_fake_listing(SEED, req.url.path()))
}

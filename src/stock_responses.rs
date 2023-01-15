use typed_html::{dom::DOMTree, elements::FlowContent, html, text, types::Metadata, OutputType};

use httpot::http::response::{Response, ResponseBuilder};

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

pub(crate) fn hello_world_response() -> Response<String> {
    let body: DOMTree<String> = boilerplate!("Hello World!", html!(<h1>"Hello, World!"</h1>));

    ResponseBuilder::default()
        .ok()
        .add_header("Server", "Apache/2.2.14 (Win32)")
        .add_header("Content-Type", "text/html")
        .version("HTTP/1.1")
        .body(body.to_string())
        .build()
        .unwrap()
}

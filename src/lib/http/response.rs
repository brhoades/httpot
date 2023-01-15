use std::fmt;
use std::string::ToString;

use crate::{http::headers::Headers, prelude::*};

#[derive(Default, Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct Response<B>
where
    B: AsRef<[u8]>,
{
    #[builder(setter(custom))]
    status_code: StatusCode,
    #[builder(setter(custom))]
    body: B,
    #[builder(setter(custom))]
    headers: Headers,

    #[builder(setter(into, strip_option), default)]
    version: Option<String>,
}

impl<B: AsRef<[u8]>> Response<B> {
    pub fn to_string(&self) -> Result<String> {
        let mut lines: Vec<String> = vec![format!(
            "{} {} {}",
            self.version
                .clone()
                .unwrap_or_else(|| "HTTP/1.1".to_string()),
            self.status_code as i32,
            self.status_code.to_string(),
        )];

        lines.extend(
            self.headers
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v.as_slice().join(", ")))
                .collect::<Vec<_>>(),
        );
        lines.push("".to_string());
        lines.push(
            String::from_utf8(self.body.as_ref().to_vec())
                .map_err(|e| anyhow!("body failed to convert to utf8: {}", e))?,
        );

        Ok(lines.as_slice().join("\r\n"))
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>> {
        self.to_string().map(|s| s.into_bytes())
    }
}

impl<B> ResponseBuilder<B>
where
    B: AsRef<[u8]>,
{
    pub fn add_headers<S: ToString>(&mut self, name: &str, values: Vec<S>) -> &mut Self {
        for v in values {
            self.add_header(name, v);
        }
        self
    }

    pub fn add_header<S: ToString>(&mut self, name: &str, value: S) -> &mut Self {
        if self.headers.is_none() {
            self.headers = Some(Headers::default());
        }
        self.headers.as_mut().unwrap().add(name, value);

        self
    }

    pub fn status_code<I: num::traits::ToPrimitive>(&mut self, status: I) -> &mut Self {
        // globally, these interfere with derive macros used for StatusCode
        use num::traits::FromPrimitive;
        self.status_code = status.to_i64().and_then(StatusCode::from_i64);
        self
    }

    pub fn body(&mut self, body: B) -> &mut Self {
        let len = body.as_ref().len();

        self.body = Some(body);
        self.add_header("Content-Length", len);
        self
    }

    pub fn ok(&mut self) -> &mut Self {
        self.status_code = Some(StatusCode::Ok);
        self
    }
}

/// Limited set of StatusCodes supported by httpot.
#[derive(Debug, PartialEq, Eq, FromPrimitive, ToPrimitive, Clone, Copy, Default)]
pub enum StatusCode {
    // 100s

    // 200s
    #[default]
    Ok = 200,
    Created,
    Accepted,
    NoContent = 204,

    // 300s
    MovedPermanently = 301,
    Found,
    SeeOther,
    TemporaryRedirect = 307,
    PermanentRedirect,

    // 400s
    Unauthorized = 401,
    Forbidden = 403,
    MethodNotAllowed = 405,
    RequestTimeout = 408,
    Gone = 410,
    LengthRequired = 411,
    PayloadTooLarge = 413,
    ImATeapot = 418,

    // 500s
    InternalServerError = 500,
    NotImplemented = 501,
    HTTPVersionNotSupported = 505,
}

impl StatusCode {
    pub fn to_string(&self) -> String {
        use StatusCode::*;

        match self {
            Ok => "OK",
            Created => "Created",
            Accepted => "Accepted",
            NoContent => "No Content",

            MovedPermanently => "Moved Permanently",
            Found => "Found",
            SeeOther => "See Other",
            TemporaryRedirect => "Temporary Redirect",
            PermanentRedirect => "Permanent Redirect",

            Unauthorized => "Unauthorized",
            Forbidden => "Forbidden",
            MethodNotAllowed => "Method Not Allowed",
            RequestTimeout => "Request Timeout",
            Gone => "Gone",
            LengthRequired => "Length Required",
            PayloadTooLarge => "Payload Too Large",
            ImATeapot => "Im A Teapot",

            InternalServerError => "Internal Server Error",
            NotImplemented => "Not Implemented",
            HTTPVersionNotSupported => "HTTP Version not Supported",
        }
        .to_string()
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

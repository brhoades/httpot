use std::string::ToString;
use std::{fmt, io::ErrorKind as IOErrorKind, sync::Arc};

use chrono::offset::Utc;
use tokio::net::TcpStream;

use crate::{http::headers::Headers, prelude::*};

#[derive(Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct BaseResponse<T: fmt::Debug> {
    #[builder(setter(custom))]
    output: T,

    #[builder(setter(custom))]
    status_code: StatusCode,
    #[builder(setter(custom))]
    body: Vec<u8>,
    #[builder(setter(custom), default = "default_headers()")]
    headers: Headers,

    #[builder(setter(into, strip_option), default)]
    version: Option<String>,
}

pub type Response = BaseResponse<Arc<TcpStream>>;
pub type ResponseBuilder = BaseResponseBuilder<Arc<TcpStream>>;

fn default_headers() -> Headers {
    let mut headers = Headers::default();
    headers.add(
        "Server",
        format!(
            "httpot{}",
            if let Ok(ver) = std::env::var("CARGO_PKG_VERSION") {
                "/".to_owned() + &ver
            } else {
                "".to_string()
            }
        ),
    );
    headers.add("Date", Utc::now().format("%a, %d %b %Y %H:%M:%S GMT"));

    headers
}

impl<T: fmt::Debug + Clone> BaseResponse<T> {
    pub fn to_string(&self) -> Result<String> {
        self.clone().into_string()
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>> {
        self.to_string().map(|s| s.into_bytes())
    }
}

impl<T: fmt::Debug> BaseResponse<T> {
    pub fn into_string(self) -> Result<String> {
        let mut lines: Vec<String> = vec![format!(
            "{} {} {}",
            self.version.unwrap_or_else(|| "HTTP/1.1".to_string()),
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
            String::from_utf8(self.body)
                .map_err(|e| anyhow!("body failed to convert to utf8: {}", e))?,
        );

        Ok(lines.as_slice().join("\r\n"))
    }

    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn len(&self) -> usize {
        self.body.len()
    }
}

impl BaseResponse<Arc<TcpStream>> {
    /// write all of the response to the connection until an error occurs
    /// or we complete.
    ///
    /// send will indefinitely loop until the connection is closed or the
    /// entire response is written. Callers should time out after an unreasonable
    /// amount of time if desired.
    pub async fn send(&mut self) -> Result<()> {
        let mut n = 0;
        let buf = self.to_string()?;
        let buf = buf.as_bytes();
        loop {
            self.output
                .writable()
                .await
                .map_err(|e| anyhow!("write half failed to be writeable in write loop: {}", e))?;
            match self.output.try_write(&buf[n..]) {
                Ok(remainder) if remainder + n < buf.len() => {
                    let new_n = n + remainder;
                    trace!(
                        "wrote only {} of remaining {} in metrics response, will retry",
                        n,
                        buf.len() - new_n
                    );
                    n = new_n;
                }
                Ok(remainder) => {
                    trace!(
                        "done writing metrics response with remainder {} (n={}, buf.len={})",
                        remainder,
                        n,
                        buf.len()
                    );
                    break;
                }
                Err(e) if e.kind() == IOErrorKind::WouldBlock => {
                    trace!("metrics response would block by writing, waiting");
                }
                Err(e) => bail!(
                    "failed to write remaining buf remainder={}, n={}, buf.len()={}: {}",
                    buf.len() - n,
                    n,
                    buf.len(),
                    e
                ),
            }
        }

        Ok(())
    }
}

impl<T: fmt::Debug> BaseResponseBuilder<T> {
    pub fn default(out: T) -> Self {
        Self {
            output: Some(out),
            status_code: Default::default(),
            body: Default::default(),
            headers: Some(default_headers()),
            version: None,
        }
    }

    pub fn ok(out: T) -> Self {
        let mut s: Self = Self::default(out);
        s.status_code = Some(StatusCode::Ok);
        s
    }

    pub fn not_found(out: T) -> Self {
        let mut s: Self = Self::default(out);
        s.status_code = Some(StatusCode::NotFound);
        s
    }

    pub fn output(&mut self, out: T) -> &mut Self {
        self.output = Some(out);
        self
    }

    pub fn add_headers<S: ToString>(&mut self, name: &str, values: Vec<S>) -> &mut Self {
        for v in values {
            self.add_header(name, v);
        }
        self
    }

    pub fn add_header<S: ToString>(&mut self, name: &str, value: S) -> &mut Self {
        if self.headers.is_none() {
            self.headers = Some(default_headers());
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

    pub fn body<B: AsRef<[u8]>>(&mut self, body: B) -> &mut Self {
        let body = body.as_ref().iter().cloned().collect::<Vec<_>>();
        let len = body.len();

        self.body = Some(body);
        self.add_header("Content-Length", len);
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
    BadRequest = 400,
    Unauthorized,
    Forbidden = 403,
    NotFound,
    MethodNotAllowed,
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

            BadRequest => "BadRequest",
            Unauthorized => "Unauthorized",
            Forbidden => "Forbidden",
            NotFound => "Not Found",
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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn basic_utf8_body() {
        let size = 1024 * 1024;
        let mut body = Vec::<u8>::with_capacity(256 * 1024 * 1024);
        body.resize(size, 'a' as u8);

        let resp = BaseResponseBuilder::ok(()).body(body).build().unwrap();
        let len: usize = resp
            .headers
            .get("Content-Length")
            .and_then(|v| v.first())
            .expect("content length header should be present")
            .parse()
            .unwrap();

        assert_eq!(len, size);
    }

    // multibyte unicode should be counted appropriately;
    #[tokio::test]
    async fn expanded_utf8_body() {
        use rand::{thread_rng, Rng};

        let body: String = thread_rng()
            .sample_iter::<char, _>(rand::distributions::Standard)
            .take(2048)
            .collect();
        let size = body.len();

        let resp = BaseResponseBuilder::ok(()).body(body).build().unwrap();
        let len: usize = resp
            .headers
            .get("Content-Length")
            .and_then(|v| v.first())
            .unwrap()
            .parse()
            .unwrap();

        assert_eq!(len, size);
    }
}

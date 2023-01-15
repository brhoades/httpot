use std::string::ToString;

use rand::prelude::*;

use crate::{prelude::*, http::headers::Headers};

#[derive(Default, Builder, Debug)]
#[builder(setter(into))]
pub struct Response<B>
where
    B: AsRef<[u8]>,
{
    #[builder(setter(custom))]
    status_code: StatusCode,
    body: B,
    #[builder(setter(custom))]
    headers: Headers,
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
        use num::traits::{ToPrimitive, FromPrimitive};
        self.status_code = status.to_i64().and_then(StatusCode::from_i64);
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


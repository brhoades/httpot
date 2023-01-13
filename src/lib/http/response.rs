use std::collections::HashMap;

use crate::http::headers::Headers;
use crate::prelude::*;

#[derive(Default, Builder, Debug)]
#[builder(setter(into))]
pub struct Response<B>
where
    B: AsRef<[u8]>,
{
    status_code: u16,
    body: B,
    headers: Headers,
}

/// return a rendered listing links provided with the same named
/// subpath.
pub fn directory_list() {}

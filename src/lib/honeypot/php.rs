use lazy_static::lazy_static;
use regex::Regex;
use url::Url;

use crate::{
    http::{
        request::Request,
        response::{Response, ResponseBuilder},
    },
    prelude::*,
};

/*
original resp:
const PHP_GIF_RESP: &str = r#"HTTP/1.1 200 OK
Date: Mon, 16 Jan 2023 17:19:06 GMT
Server: Apache/2.2.22 (Ubuntu)
X-Powered-By: PHP/4.0.1
Expires: Thu, 19 Nov 1981 08:52:00 GMT
Cache-Control: no-store, no-cache, must-revalidate, post-check=0, pre-check=0
Pragma: no-cache
Vary: Accept-Encoding
Content-Encoding: gzip
Content-Length: 2985
Keep-Alive: timeout=5, max=100
Connection: Keep-Alive
Content-Type: image/gif"#;
*/

pub fn is_easter_egg(req: &Request) -> bool {
    is_easter_egg_url(&req.url)
}

lazy_static! {
    static ref RE: Regex = Regex::new(
        "PHP[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}"
    )
    .unwrap();
}

fn is_easter_egg_url(url: &Url) -> bool {
    url.query_pairs()
        .any(|(k, v)| k == "" && RE.is_match(v.as_ref()))
}

/// Returns a php easter egg response relevant to the requested easter egg.
/// An error is returned if no known easter egg is requested.
pub fn easter_egg(req: &Request) -> Result<Response> {
    let (_, v) = req.url.query_pairs().find(|(k, v)| k == "" && RE.is_match(v)).ok_or_else(|| anyhow!("failed to find PHP easter egg queryparam in order to build easter egg response in url: {}", req.url))?;

    match v.as_ref() {
        "PHPE9568F36-D428-11d2-A769-00AA001ACF42" => php_image_resp(),
        "PHPE9568F34-D428-11d2-A769-00AA001ACF42" => php_image_resp(),
        "PHPE9568F35-D428-11d2-A769-00AA001ACF42" => php_image_resp(),
        "PHPB8B5F2A0-3C92-11d3-A3A9-4C7B08C10000" => php_credits(),
        v => bail!(
            "unknown php easter egg querystring '{}' in url '{}'",
            v,
            req.url
        ),
    }
}

// the image responses are different, but I doubt scrapers are sniffing for content length or rendering
// them
fn php_image_resp() -> Result<Response> {
    Ok(ResponseBuilder::ok()
        .body((0..2985).map(|_| 'a' as u8).collect::<Vec<u8>>())
        .add_header("Content-Type", "image/gif")
        .add_header("X-Powered-By", "PHP/4.0.1")
        .build()?)
}

// php 4.4.0 credits html though pretending to be an earlier version
fn php_credits() -> Result<Response> {
    Ok(ResponseBuilder::ok()
        .body(include_str!("php_credits.html"))
        .add_header("Content-Type", "text/html")
        .add_header("X-Powered-By", "PHP/4.0.1")
        .build()?)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_php_easter_egg_hit() {
        let pos_cases = vec![
            "http://example.com/?=PHPE9568F36-D428-11d2-A769-00AA001ACF42",
            "http://example.com/?=PHPB8B5F2A0-3C92-11d3-A3A9-4C7B08C10000",
            "https://example.com/?=PHPE9568F35-D428-11d2-A769-00AA001ACF42",
            "http://192.168.1.1/?=PHPE9568F34-D428-11d2-A769-00AA001ACF42",
            "http://example.com/foobar.php?=PHPE9568F34-D428-11d2-A769-00AA001ACF42",
        ];

        for c in pos_cases {
            assert!(
                is_php_easter_egg_url(&Url::parse(c).unwrap()),
                "failed to match url: {}",
                c
            );
        }

        let neg_cases = vec![
            "https://google.com",
            "https://google.com/",
            "https://google.com/foobar/baz/bim?1=2&3=4&6=&=5",
            "http://brod.es/?foo=bar&biz=baz",
        ];

        for c in neg_cases {
            assert!(
                !is_php_easter_egg_url(&Url::parse(c).unwrap()),
                "failed to not match url: {}",
                c
            );
        }
    }
}

use crate::http::headers::Headers;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use url::Url;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Request {
    pub headers: Headers,
    pub size: usize,
    pub body: Vec<u8>,
    pub method: Method,
    pub url: Url,
    pub version: String,
    pub remote_ip: SocketAddr,
}

#[derive(Debug, Default)]
enum RequestReadState {
    #[default]
    Version,
    Headers,
    Body,
}

pub async fn parse_request<T: std::marker::Unpin + AsyncBufReadExt>(
    addr: &SocketAddr,
    reader: &mut T,
) -> Result<Request> {
    let mut version = None;
    let mut method: Option<Method> = None;
    let mut headers = Headers::default();
    let mut path = None;
    let mut body_len = None;
    let mut body = Vec::<u8>::new();
    let remote_addr = addr; // TODO: handle X-Forwarded-For / X-Real-IP

    let mut state = RequestReadState::Version;
    'request: loop {
        state = match state {
            RequestReadState::Version => {
                let mut line: String = "".to_string();
                reader.read_line(&mut line).await.map_err(|e| {
                    anyhow!("request ended early when reading version with error: {}", e)
                })?;

                let fragments = line.split(" ").collect::<Vec<_>>();
                match fragments.as_slice() {
                    &[m, p, v] => {
                        method = Some(m.parse()?);
                        path = Some(p.to_string());
                        version = Some(v.to_string());
                    }
                    other => bail!("unknown http opening line: {:?}", other),
                }

                debug!(
                    "got inital paramters: {:?} {:?} w/ version {:?}",
                    method, path, version
                );
                RequestReadState::Headers
            }
            RequestReadState::Headers => {
                let mut line: String = "".to_string();
                reader.read_line(&mut line).await.map_err(|e| {
                    anyhow!("request ended early when reading version with error: {}", e)
                })?;

                match line.split_once(":") {
                    None => {
                        debug!("done reading header: '{:?}'", line);
                        RequestReadState::Body
                    } // presumptive done?
                    Some((name, val)) => {
                        let val = val.trim();

                        if name.to_lowercase() == "content-length" {
                            body_len = Some(val.parse::<usize>()?);
                        }

                        let vals = val
                            .split(",")
                            .map(|s| s.trim().to_string())
                            .collect::<Vec<_>>();

                        debug!("added headers: {} => {:?}", name, vals);

                        headers
                            .entry(name.to_string())
                            .and_modify(|v: &mut Vec<String>| v.extend_from_slice(vals.as_slice()))
                            .or_insert(vals.iter().map(|s| s.to_string()).collect());
                        RequestReadState::Headers
                    }
                }
            }
            RequestReadState::Body => {
                debug!("reading body of method: {:?}", method);
                use Method::*;
                match method.as_ref() {
                    Some(GET) | Some(HEAD) | Some(DELETE) | Some(CONNECT) | Some(OPTIONS)
                    | Some(TRACE) => {
                        debug!("finished reading body for method: {:?}", method);
                    }

                    Some(_) if body_len.is_some() => {
                        let len = body_len.as_ref().unwrap();
                        body = Vec::with_capacity(*len);
                        body.resize(*len, 0);
                        debug!("reading body of size {}", len);
                        reader
                            .read(&mut body)
                            .await
                            .map_err(|e| anyhow!("failed to read body with len {}: {}", len, e))?;

                        debug!("read body len={}: {:?}", body.len(), body);
                    }
                    Some(method) => debug!("skipping body for {:?}", method),
                    None => bail!("request lacked method"),
                };
                break 'request;
            }
        };
    }

    debug!("req done");
    let url = format!(
        "http://{}{}",
        headers
            .get("Host")
            .and_then(|v| v.first())
            .ok_or_else(|| anyhow!("failed to get host header"))?,
        path.ok_or_else(|| anyhow!("did not get path"))?
    );

    debug!("urlstr: {}", url);
    let url = Url::parse(&url).map_err(|e| anyhow!("failed to construct url: {}", e))?;
    let req = Request {
        headers,
        size: body_len.unwrap_or_default(),
        url,
        body,
        method: method.unwrap_or_default(),
        version: version.unwrap_or_default().trim().to_string(),
        remote_ip: remote_addr.to_owned(),
    };

    debug!("done reading request. url: {}. req: {:?}", req.url, req);
    Ok(req)
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum Method {
    GET,
    #[default]
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
}

impl std::str::FromStr for Method {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        use Method::*;

        Ok(match s {
            "GET" => GET,
            "HEAD" => HEAD,
            "POST" => POST,
            "PUT" => PUT,
            "DELETE" => DELETE,
            "CONNECT" => CONNECT,
            "OPTIONS" => OPTIONS,
            "TRACE" => TRACE,
            other => bail!("unknown HTTP method: {}", other),
        })
    }
}

impl Method {
    pub fn to_string(&self) -> String {
        use Method::*;
        match self {
            GET => "GET",
            HEAD => "HEAD",
            POST => "POST",
            PUT => "PUT",
            DELETE => "DELETE",
            CONNECT => "CONNECT",
            OPTIONS => "OPTIONS",
            TRACE => "TRACE",
        }
        .to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::BufReader;
    use url::Host;

    #[tokio::test]
    async fn test_basic_get_request_parse() {
        let input = r#"GET / HTTP/1.1
Host: 127.0.0.1:8080
User-Agent: curl/7.83.1
Foo: Bar
Biz: Baz
Cookie: asdf=123, fghj=4567, session=someid
Accept: */*

"#;
        let mut r = BufReader::new(input.as_bytes());

        let req = parse_request(&mut r).await.unwrap();

        assert_eq!(Method::GET, req.method);
        assert_eq!("/", req.url.path());
        assert_eq!("127.0.0.1".parse().ok().map(Host::Ipv4), req.url.host());
        assert_eq!(8080, req.url.port().unwrap_or_default());
        assert_eq!("HTTP/1.1", req.version);
        assert_eq!(0, req.body.len());

        let cases = vec![
            ("Host", vec!["127.0.0.1:8080"]),
            ("User-Agent", vec!["curl/7.83.1"]),
            ("Foo", vec!["Bar"]),
            ("Biz", vec!["Baz"]),
            ("Cookie", vec!["asdf=123", "fghj=4567", "session=someid"]),
            ("Accept", vec!["*/*"]),
        ];
        assert_headers_eq(cases, &req.headers);
    }

    #[tokio::test]
    async fn test_basic_post_request_parse() {
        let input = r#"POST / HTTP/1.1
Accept: application/json, */*;q=0.5
Accept-Encoding: gzip, deflate, br
Connection: keep-alive
Content-Length: 29
Content-Type: application/json
Host: 127.0.0.1:8080
User-Agent: HTTPie/3.2.1

{
    "foo": "bar",
    "test": "baz"
}
"#;
        let mut r = BufReader::new(input.as_bytes());

        let req = parse_request(&mut r).await.unwrap();

        assert_eq!(Method::POST, req.method);
        assert_eq!("/", req.url.path());
        assert_eq!("127.0.0.1".parse().ok().map(Host::Ipv4), req.url.host());
        assert_eq!(8080, req.url.port().unwrap_or_default());
        assert_eq!("HTTP/1.1", req.version);
        assert_eq!(29, req.body.len());

        let cases = vec![
            ("Host", vec!["127.0.0.1:8080"]),
            ("User-Agent", vec!["HTTPie/3.2.1"]),
            ("Accept", vec!["application/json", "*/*;q=0.5"]),
            ("Accept-Encoding", vec!["gzip", "deflate", "br"]),
            ("Connection", vec!["keep-alive"]),
            ("Content-Length", vec!["29"]),
            ("Content-Type", vec!["application/json"]),
        ];
        assert_headers_eq(cases, &req.headers);
    }

    fn assert_headers_eq(expected: Vec<(&str, Vec<&str>)>, actual: &Headers) {
        assert_eq!(expected.len(), actual.len());

        for (header, expected) in expected {
            let expected = expected.into_iter().map(|s| s.to_string()).collect();
            assert_eq!(
                Some(&expected),
                actual.get(header),
                "expected header '{}' to have value '{:?}', but had value '{:?}'",
                header,
                expected,
                actual.get(header)
            );
        }
    }
}

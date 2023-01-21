use std::fmt;
use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use url::Url;

use crate::{
    http::headers::{self, Headers},
    prelude::*,
};

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
    let remote_addr = addr;

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
                        let vals =
                            match headers::KNOWN_LIST_HEADERS.get(name.to_lowercase().as_str()) {
                                Some(delim) => val
                                    .split(delim)
                                    .map(|s| s.trim().to_string())
                                    .collect::<Vec<_>>(),
                                None => vec![val.to_string()],
                            };

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

impl Request {
    /// Provides the proxy-aware requesting address, the first value in this
    /// order that parses as a SocketAddr is accepted:
    ///  * for in "Forwarded"
    ///  * X-Forwarded-For
    ///  * self.remote_ip
    pub fn requester(&self) -> String {
        let forwarded = self
            .headers
            .get("Forwarded")
            .and_then(|v| v.first())
            .and_then(|vals| {
                vals.split(|c| c == ',' || c == ';')
                    .map(|s| s.to_lowercase())
                    .filter_map(|pair| match pair.split_once('=') {
                        Some((k, v)) if k.trim() == "for" => Some(v.trim().to_string()),
                        _ => None,
                    })
                    .next()
            });

        if let Some(fwd) = forwarded {
            return fwd;
        }

        let forwarded = self
            .headers
            .get("X-Forwarded-For")
            .and_then(|v| v.first())
            .and_then(|vals| vals.split(",").map(|v| v.trim()).next());

        if let Some(fwd) = forwarded {
            return fwd.to_string();
        }

        self.remote_ip.to_string()
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut lines = vec![
            format!(
                "{} {} {}",
                self.method.to_string(),
                self.url.path(),
                self.version,
            ),
            self.headers.to_string(),
            "".to_string(),
        ];

        if self.size > 0 {
            lines.push(String::from_utf8(self.body.clone()).map_err(|e| {
                error!("body failed to convert request body to utf8: {}", e);
                fmt::Error
            })?);
        }

        write!(f, "{}", lines.join("\n\r"))
    }
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
Cookie: asdf=123; fghj=4567;session=someid
Accept: */*

"#;
        let mut r = BufReader::new(input.as_bytes());
        let peer = "127.0.0.1:8000".parse().unwrap();

        let req = parse_request(&peer, &mut r).await.unwrap();

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
    async fn test_requester() {
        let _ = pretty_env_logger::try_init();
        let mut req = stub_request();
        req.remote_ip = "1.2.3.4:61723".parse().unwrap();

        // expected => vec of headers to apply
        let cases = vec![
            ("1.2.3.4:61723", vec![]),
            (
                "192.168.1.100:50212",
                vec![("X-Forwarded-For", "192.168.1.100:50212")],
            ),
            ("192.168.1.100", vec![("X-Forwarded-For", "192.168.1.100")]),
            (
                "192.168.162.109:46591",
                vec![
                    ("X-Forwarded-For", "192.168.1.251"),
                    ("Forwarded", "for=192.168.162.109:46591"),
                ],
            ),
            (
                "192.168.162.109",
                vec![("Forwarded", "for=192.168.162.109")],
            ),
            (
                "203.0.113.195",
                vec![(
                    "X-Forwarded-For",
                    "203.0.113.195, 2001:db8:85a3:8d3:1319:8a2e:370:7348",
                )],
            ),
            (
                "210.0.113.195",
                vec![(
                    "X-Forwarded-For",
                    "210.0.113.195,2001:db8:85a3:8d3:1319:8a2e:370:7348",
                )],
            ),
        ];
        for (i, (expected, headers)) in cases.into_iter().enumerate() {
            let mut req = req.clone();
            for (k, v) in headers {
                req.headers.add(k, v);
            }

            assert_eq!(
                expected.to_string(),
                req.requester(),
                "case i={}: headers did not yield correct requester addr",
                i
            );
        }
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

    fn stub_request() -> Request {
        Request {
            headers: Headers::new(),
            size: 0,
            body: vec![],
            method: Method::GET,
            url: "http://127.0.0.1:8080/".parse().unwrap(),
            version: "HTTP/1.1".to_string(),
            remote_ip: "1.1.1.1:62012".parse().unwrap(),
        }
    }
}

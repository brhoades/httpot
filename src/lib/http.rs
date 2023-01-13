use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use url::Url;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Request {
    pub headers: HashMap<String, Vec<String>>,
    pub size: usize,
    pub body: Vec<u8>,
    pub method: Method,
    pub url: Url,
}

#[derive(Debug, Default)]
enum RequestReadState {
    #[default]
    Version,
    Headers,
    Body,
}

pub async fn parse_request<T: std::marker::Unpin + AsyncBufReadExt>(
    reader: &mut T,
) -> Result<Request> {
    warn!("PRASE REQ");
    // let mut host = None;
    let mut version = None;
    let mut method: Option<Method> = None;
    // let mut url = None;
    let mut headers = HashMap::<String, Vec<String>>::default();
    let mut path = None;
    let mut body_len = None;
    let mut body = Vec::<u8>::new();

    let mut state = RequestReadState::Version;
    'request: loop {
        state = match state {
            RequestReadState::Version => {
                debug!("reading http version");
                let mut line: String = "".to_string();
                debug!("getting version line");
                reader.read_line(&mut line).await.map_err(|e| {
                    anyhow!("request ended early when reading version with error: {}", e)
                })?;
                debug!("got line: {}", line);

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
                debug!("getting header line");
                reader.read_line(&mut line).await.map_err(|e| {
                    anyhow!("request ended early when reading version with error: {}", e)
                })?;
                debug!("got header line: {}", line);

                match line.split_once(":") {
                    None => {
                        info!("done reading header: '{:?}'", line);
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
    };

    debug!("done reading request. url: {}. req: {:?}", req.url, req);
    Ok(req)
}

#[derive(Debug, Clone, Default)]
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

mod test {
    use super::*;

    #[test]
    async fn test_boring_http_get() {
        let mut input = r#"GET / HTTP/1.1
Host: 127.0.0.1:8080
User-Agent: curl/7.83.1
Accept: */*

"#;
    }
}

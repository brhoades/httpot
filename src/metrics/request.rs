use lazy_static::lazy_static;
use std::time::Instant;

use prometheus::{self as prom, register_counter_vec, register_histogram, register_histogram_vec};
use std::future::Future;

use httpot::{http::request::Request, prelude::*};

lazy_static! {
    pub static ref HTTP_REQUEST: prom::HistogramVec = register_histogram_vec!(
        "http_request",
        "Incoming HTTP request read and parse time",
        &["method", "remote_addr", "user_agent", "version"]
    )
    .unwrap();
    pub static ref HTTP_REQUEST_PARSE_FAILURES: prom::Histogram = register_histogram!(
        "http_request_parse_failures",
        "Incoming HTTP request parse failures time",
    )
    .unwrap();
    pub static ref HTTP_REQUEST_BODY: prom::CounterVec = register_counter_vec!(
        "http_request_body_size",
        "Incoming HTTP request cumulative body size",
        &["method", "remote_addr", "user_agent", "version"]
    )
    .unwrap();
    pub static ref HTTP_REQUEST_PATH_LENGTH: prom::CounterVec = register_counter_vec!(
        "http_request_path_length",
        "Incoming HTTP request cumulative request path length",
        &["method", "remote_addr", "user_agent", "version"]
    )
    .unwrap();
}

pub async fn observe_request<R: Future<Output = Result<Request>>>(req: R) -> Result<Request> {
    let start = Instant::now();
    let req = req.await;
    let elapsed = start.elapsed().as_secs_f64();

    if req.is_err() {
        HTTP_REQUEST_PARSE_FAILURES.observe(elapsed);
        return req;
    }

    let req = req?;
    let ip = req.requester().to_string();
    let meth = req.method.to_string();

    let common_labels: Vec<&str> = vec![
        &meth,
        &ip,
        req.headers
            .get("User-Agent")
            .and_then(|s| s.first())
            .map(|s| s.as_str())
            .unwrap_or_else(|| "unknown"),
        &req.version,
    ];

    HTTP_REQUEST
        .with_label_values(common_labels.as_slice())
        .observe(elapsed);

    HTTP_REQUEST_BODY
        .with_label_values(common_labels.as_slice())
        .inc_by(req.size as f64);

    HTTP_REQUEST_PATH_LENGTH
        .with_label_values(common_labels.as_slice())
        .inc_by(req.size as f64);

    Ok(req)
}

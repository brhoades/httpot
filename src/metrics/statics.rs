use lazy_static::lazy_static;
use prometheus::{self as prom, register_counter_vec, register_histogram_vec};

lazy_static! {
    pub static ref HTTP_REQUEST: prom::HistogramVec = register_histogram_vec!(
        "http_request",
        "Incoming HTTP request read and parse time",
        &["method", "remote_ip", "user_agent", "version"]
    )
    .unwrap();
    pub static ref HTTP_REQUEST_BODY: prom::CounterVec = register_counter_vec!(
        "http_request_body_size",
        "Incoming HTTP request cumulative body size",
        &["method", "remote_ip", "user_agent", "version"]
    )
    .unwrap();
    pub static ref HTTP_REQUEST_PATH_LENGTH: prom::CounterVec = register_counter_vec!(
        "http_request_path_length",
        "Incoming HTTP request cumulative request path length",
        &["method", "remote_ip", "user_agent", "version"]
    )
    .unwrap();
}

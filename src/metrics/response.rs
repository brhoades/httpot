use lazy_static::lazy_static;

use prometheus::{self as prom, register_counter_vec, register_histogram_vec};

lazy_static! {
    pub static ref HTTP_RESPONSE: prom::HistogramVec = register_histogram_vec!(
        "httpot_http_response",
        "Outgoing HTTP response render and write time",
        &["method", "remote_addr", "user_agent", "version", "route"]
    )
    .unwrap();
    pub static ref HTTP_RESPONSE_RENDER_FAILURES: prom::HistogramVec = register_histogram_vec!(
        "httpot_http_response_render_failures",
        "Outgoing HTTP response render failures time",
        &[
            "method",
            "path",
            "remote_addr",
            "user_agent",
            "version",
            "route"
        ]
    )
    .unwrap();
    pub static ref HTTP_RESPONSE_BODY: prom::CounterVec = register_counter_vec!(
        "httpot_http_response_body_size",
        "Outoing HTTP response cumulative body size",
        &["method", "remote_addr", "user_agent", "version", "route"]
    )
    .unwrap();
}

/*
pub async fn observe_response<R: Future<Output = Result<Response>>>(resp: R) -> Result<Response> {
    let start = Instant::now();
    let resp = resp.await;
    let elapsed = start.elapsed().as_secs_f64();

    if resp.is_err() {
        HTTP_RESPONSE_RENDER_FAILURES.observe(elapsed);
        return resp;
    }

    let resp = resp?;
    let ip = resp.respuester().to_string();
    let meth = resp.method.to_string();

    let common_labels: Vec<&str> = vec![
        &meth,
        &ip,
        &resp
            .headers
            .get_all(&vec!["User-Agent", "user-agent"])
            .into_iter()
            .next()
            .map(|v| v.as_str())
            .unwrap_or_else(|| "unknown"),
        &resp.version,
    ];

    HTTP_RESPUEST
        .with_label_values(common_labels.as_slice())
        .observe(elapsed);

    HTTP_RESPUEST_BODY
        .with_label_values(common_labels.as_slice())
        .inc_by(resp.size as f64);

    HTTP_RESPUEST_PATH_LENGTH
        .with_label_values(common_labels.as_slice())
        .inc_by(resp.url.path().len() as f64);

    Ok(resp)
}

*/

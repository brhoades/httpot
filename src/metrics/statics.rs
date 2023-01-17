use lazy_static::lazy_static;
use prometheus::{
    self, default_registry, register_histogram_vec, register_int_counter, Counter, Error,
    HistogramVec, IntCounter, Opts,
};

use httpot::prelude::*;

lazy_static! {
    pub static ref HIGH_FIVE_COUNTER: IntCounter =
        register_int_counter!("http.request", "").unwrap();
    pub static ref HTTP_REQUEST: HistogramVec = register_histogram_vec!(
        "http.request",
        "Incoming HTTP request read and parse time",
        &[
            "method",
            "body_size",
            "remote_ip",
            "path",
            "host",
            "user_agent",
            "version",
        ]
    )
    .unwrap();
}

pub fn count(name: &str, desc: &str) -> Result<()> {
    let opts = Opts::new(name, desc);
    let counter = Counter::with_opts(opts)?;

    let r = default_registry();
    match r.register(Box::new(counter.clone())) {
        Ok(_) => (),
        Err(Error::AlreadyReg) => (),
        Err(e) => bail!("failed register metric: {}", e),
    }
    counter.inc();

    Ok(())
}

/*
#[macro_export]
macro_rules! count {
}

*/

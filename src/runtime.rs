use std::env;

use log::LevelFilter;
use pretty_env_logger::env_logger::Target;
use tokio::signal::{
    ctrl_c,
    unix::{signal, SignalKind},
};

use httpot::prelude::*;

/// catches sigterm and ctrl+c, exiting when received
pub(crate) async fn interrupt() -> Result<()> {
    let mut term = signal(SignalKind::terminate())?;

    tokio::select!(
        _ = term.recv() => (),
        res = ctrl_c() => res?,
    );

    Ok(())
}

pub(crate) fn logging(level: &Option<LevelFilter>, target: &Target) {
    let mut builder = pretty_env_logger::formatted_timed_builder();
    let level = if let Some(lvl) = level {
        Some(lvl.clone())
    } else if env::var("RUST_LOG").is_ok() {
        // set by default
        None
    } else {
        Some(LevelFilter::Info)
    };

    if let Some(lvl) = level {
        builder.filter_level(lvl);
    }

    let res = builder.target(*target).try_init();

    match res {
        Err(e) => println!("failed to init {}", e),
        Ok(_) => warn!("logger initialized at level={:?}", level),
    }
}

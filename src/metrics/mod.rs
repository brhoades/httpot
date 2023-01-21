// heavy inspiration from:
// https://romankudryashov.com/blog/2021/11/monitoring-rust-web-application/

mod request;
mod statics;

pub use request::*;
pub use statics::*;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use prometheus::TextEncoder;
use tokio::{
    io::BufReader,
    net::tcp::OwnedWriteHalf,
    net::{TcpListener, TcpStream},
    time::sleep,
};

use httpot::{
    http::{
        request::{parse_request, Method, Request},
        response::{ResponseBuilder, StatusCode},
        stock_responses,
    },
    prelude::*,
};

/// self-disables and sleeps indefintiely on None. Otherwise listens
/// for incoming requests and returns prometheus metrics.
pub async fn run(addr: Option<SocketAddr>) -> Result<()> {
    if addr.is_none() {
        sleep(Duration::MAX).await;
    }

    let addr = addr.unwrap();
    let l = TcpListener::bind(&addr).await?;
    info!("metrics listening on: {}", addr);

    loop {
        let socket = match l.accept().await {
            Err(e) => {
                warn!("error when accepting metrics conn: {}", e);
                continue;
            }
            Ok((s, _)) => s,
        };

        tokio::spawn(async move {
            if let Err(e) = process_req(socket).await {
                warn!("failed to process metrics req: {}", e);
            }
        });
    }
}

async fn process_req(s: TcpStream) -> Result<()> {
    let addr = s.peer_addr()?;
    let (r, mut w) = s.into_split();
    debug!("metrics conn from {}", addr);

    r.readable().await?;

    let req = parse_request(&addr, &mut BufReader::new(r)).await?;
    if req.url.path() != "/" || req.method != Method::GET {
        warn!(
            "from {} => only reqs to / are supported, got {} {}",
            addr,
            req.method.to_string(),
            req.url
        );

        return four_hundred(&mut w).await;
    }

    let encoder = TextEncoder::new();
    let mut buffer = String::new();
    encoder
        .encode_utf8(&prometheus::gather(), &mut buffer)
        .expect("Failed to encode metrics");

    let resp = ResponseBuilder::ok()
        .add_header("Content-Type", "text/plain")
        .body(buffer)
        .build()?;

    w.writable().await?;
    let res = w.try_write(&resp.as_bytes()?)?;
    debug!("wrote {} metrics bytes", res);

    Ok(())
}

async fn four_hundred(w: &mut OwnedWriteHalf) -> Result<()> {
    let resp = stock_responses::generic_status(StatusCode::BadRequest).build()?;
    w.try_write(&resp.as_bytes()?)?;
    Ok(())
}

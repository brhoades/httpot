// heavy inspiration from:
// https://romankudryashov.com/blog/2021/11/monitoring-rust-web-application/

mod request;
mod response;

pub use request::*;
pub use response::*;

use std::time::Duration;

use prometheus::TextEncoder;
use tokio::{
    io::BufReader,
    net::{TcpListener, TcpStream},
    time::sleep,
};

use httpot::{
    http::{
        request::{parse_request, Method},
        response::{ResponseBuilder, StatusCode},
        stock_responses,
    },
    prelude::*,
};

/// self-disables and sleeps indefintiely on None. Otherwise listens
/// for incoming requests and returns prometheus metrics.
pub async fn run(addr: Option<std::net::SocketAddr>) -> Result<()> {
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

async fn process_req(mut s: TcpStream) -> Result<()> {
    let addr = s.peer_addr()?;
    debug!("metrics conn from {}", addr);

    s.readable().await?;

    let req = parse_request(&addr, &mut BufReader::new(&mut s)).await?;
    if (req.url.path() != "/" && req.url.path() != "/metrics") || req.method != Method::GET {
        warn!(
            "from {} => only reqs to / and /metrics are supported, got {} {}",
            addr,
            req.method.to_string(),
            req.url
        );

        return four_hundred(s).await;
    }

    s.writable().await?;

    let resp = TextEncoder::new()
        .encode_to_string(&prometheus::gather())
        .map_err(|e| anyhow!("failed to convert metrics to string: {}", e))?;

    let mut resp = ResponseBuilder::ok(std::sync::Arc::new(s))
        .add_header("Content-Type", "text/plain")
        .body(resp)
        .build()?;

    tokio::select!(
        _ = sleep(Duration::from_secs(5)) => {
            bail!("metrics response write timed out after 5 seconds");
        },
        res = resp.send() => {
            match res {
                Ok(_) => info!("{}: wrote {} metrics bytes", addr, resp.len()),
                Err(e) => {
                    bail!("{}: failed to write {} metrics bytes: {}", addr, resp.len(), e);
                }
            }
        }
    );

    Ok(())
}

async fn four_hundred(w: TcpStream) -> Result<()> {
    stock_responses::generic_status(w, StatusCode::BadRequest)
        .build()?
        .send()
        .await
}

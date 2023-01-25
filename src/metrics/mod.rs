// heavy inspiration from:
// https://romankudryashov.com/blog/2021/11/monitoring-rust-web-application/

mod request;
mod response;

pub use request::*;
pub use response::*;

use std::time::Duration;
use std::{io, net::SocketAddr};

use prometheus::TextEncoder;
use tokio::{
    io::BufReader,
    net::tcp::OwnedWriteHalf,
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
    if (req.url.path() != "/" && req.url.path() != "/metrics") || req.method != Method::GET {
        warn!(
            "from {} => only reqs to / and /metrics are supported, got {} {}",
            addr,
            req.method.to_string(),
            req.url
        );

        return four_hundred(&mut w).await;
    }

    w.writable().await?;

    let resp = TextEncoder::new()
        .encode_to_string(&prometheus::gather())
        .map_err(|e| anyhow!("failed to convert metrics to string: {}", e))?;

    let resp = ResponseBuilder::ok()
        .add_header("Content-Type", "text/plain")
        .body(resp)
        .build()?;
    let resp_body = &resp.as_bytes()?;

    tokio::select!(
        _ = sleep(Duration::from_secs(5)) => {
            bail!("metrics response write timed out after 5 seconds");
        },
        res = write_all(&mut w, &resp_body) => {
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

async fn four_hundred(w: &mut OwnedWriteHalf) -> Result<()> {
    let resp = stock_responses::generic_status(StatusCode::BadRequest).build()?;
    w.try_write(&resp.as_bytes()?)?;
    Ok(())
}

/// try to write all of the provided buffer repeatedly, waiting
/// for the read side to be ready after each attempt.
///
/// write_all will indefinitely loop until the connection is closed or the
/// entire response is written. Callers should time out after an unreasonable
/// amount of time.
async fn write_all(w: &mut OwnedWriteHalf, buf: &[u8]) -> Result<()> {
    let mut n = 0;
    loop {
        w.writable()
            .await
            .map_err(|e| anyhow!("write half failed to be writeable in write loop: {}", e))?;
        match w.try_write(&buf[n..]) {
            Ok(remainder) if remainder + n < buf.len() => {
                let new_n = n + remainder;
                debug!(
                    "wrote only {} of remaining {} in metrics response, will retry",
                    n,
                    buf.len() - new_n
                );
                n = new_n;
            }
            Ok(remainder) => {
                debug!(
                    "done writing metrics response with remainder {} (n={}, buf.len={})",
                    remainder,
                    n,
                    buf.len()
                );
                break;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                trace!("metrics response would block by writing, waiting");
            }
            Err(e) => bail!(
                "failed to write remaining buf remainder={}, n={}, buf.len()={}: {}",
                buf.len() - n,
                n,
                buf.len(),
                e
            ),
        }
    }

    Ok(())
}

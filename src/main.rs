mod router;
mod runtime;
mod stock_responses;

use log::LevelFilter;
use pretty_env_logger::env_logger::Target;
use structopt::StructOpt;

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

use httpot::{http::request, prelude::*};

#[derive(Debug, Clone, StructOpt)]
#[structopt(name = "httpot", about = "HTTP [honeyp]ot")]
struct Opt {
    #[structopt(long = "log-level", short = "l")]
    log_level: Option<LevelFilter>,

    #[structopt(long = "log-target", default_value = "stderr", parse(try_from_str = httpot::util::logtarget_parse))]
    log_target: Target,

    listen_addr: std::net::SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();
    runtime::logging(&opt.log_level, &opt.log_target);

    info!("listening on {}", &opt.listen_addr);

    let listener = TcpListener::bind(opt.listen_addr).await?;

    loop {
        let (socket, _) = tokio::select!(
            socket_res = listener.accept() => socket_res?,
            _ = runtime::interrupt() => {
                warn!("signal received");
                return Ok(());
            }
        );

        tokio::spawn(async move {
            let remote = socket
                .peer_addr()
                .map(|s| s.to_string())
                .unwrap_or_else(|e| format!("unknown addr '{}'", e));

            match process_socket(socket).await {
                Ok(_) => info!("session with {} ended successfully", remote),
                Err(e) => info!("session with {} errored: {}", remote, e),
            }
        });
    }
}

async fn process_socket(s: TcpStream) -> Result<()> {
    let addr = s.peer_addr()?.to_string();

    let (r, w) = s.into_split();

    let mut r = BufReader::new(r);
    debug!("get socket start...");
    r.get_ref().readable().await?;
    let req = request::parse_request(&mut r).await?;
    info!(
        "{: <8} {: <20} ==> {: <8} {} bytes {}",
        addr,
        truncate(
            &req.headers
                .get("User-Agent")
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            20
        ),
        req.method.to_string(),
        req.body.len(),
        truncate(req.url.path(), 20),
    );

    let resp = router::router(&req).await?;

    w.try_write(&resp.as_bytes()?)?;

    info!(
        "{: <8} <== {: <4} {: >8} bytes",
        addr,
        resp.status_code().to_string(),
        resp.len(),
    );

    // close conn
    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars - 3 {
        return s.to_string();
    }

    format!(
        "{}...{}",
        s[0..(max_chars - 3) / 2].to_string(),
        s[(s.len() - (max_chars - 3) / 2)..s.len()].to_string()
    )
}

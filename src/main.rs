use log::LevelFilter;
use pretty_env_logger::env_logger::Target;
use structopt::StructOpt;

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

use httpot::prelude::*;

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
    init_logging(&opt);

    info!("listening on {}", &opt.listen_addr);

    let listener = TcpListener::bind(opt.listen_addr).await?;

    loop {
        let (socket, _) = listener.accept().await?;

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

fn init_logging(opt: &Opt) {
    let mut builder = pretty_env_logger::formatted_timed_builder();
    let level = if let Some(lvl) = opt.log_level {
        lvl
    } else if let Ok(lvl) = std::env::var("RUST_LOG") {
        lvl.parse().unwrap()
    } else {
        LevelFilter::Info
    };

    builder.filter_level(level);

    let res = builder.target(opt.log_target).try_init();

    match res {
        Err(e) => println!("failed to init {}", e),
        Ok(_) => warn!("logger initialized at level={}", level),
    }
}

async fn process_socket(s: TcpStream) -> Result<()> {
    let (r, w) = s.into_split();

    let mut r = BufReader::new(r);
    info!("get socket start...");
    loop {
        r.get_ref().readable().await?;
        let req = httpot::http::request::parse_request(&mut r).await;
        trace!("req: {:#?}", req);

        req?;

        let body = r#"<html>
  <body>
    <h1>Hello, World!</h1>
  </body>
</html>
"#;

        let resp = r#"HTTP/1.1 200 OK
Server: Apache/2.2.14 (Win32)
Content-Length: "#
            .to_owned()
            + &body.len().to_string()
            + r#"
Content-Type: text/html

"# + body;

        info!("write resp");
        let res = w.try_write(&resp.as_bytes());
        info!("wrote resp with result: {:?}", res);
        res?;
    }
}

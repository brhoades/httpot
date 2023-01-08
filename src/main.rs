use log::LevelFilter;
use pretty_env_logger::env_logger::Target;
use structopt::StructOpt;

use httpot::prelude::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "httpot", about = "HTTP [honeyp]ot")]
struct Opt {
    #[structopt(long = "log-level", short = "l")]
    log_level: Option<LevelFilter>,

    #[structopt(long = "log-target", default_value = "stderr", parse(try_from_str = httpot::util::logtarget_parse))]
    log_target: Target,
}

fn main() {
    let opt = Opt::from_args();
    init_logging(&opt);

    info!("Hello, world!");
}

fn init_logging(opt: &Opt) {
    let mut b = pretty_env_logger::formatted_builder();

    if std::env::var("RUST_LOG").is_err() {
        if let Some(lvl) = opt.log_level {
            b.filter_level(lvl);
        } else {
            b.filter_level(LevelFilter::Info);
        }
    }
    b.target(opt.log_target).init();
}

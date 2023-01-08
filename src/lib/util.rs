use super::prelude::*;
use pretty_env_logger::env_logger::Target;

pub fn logtarget_parse(s: &str) -> Result<Target> {
    Ok(match s {
        "stdout" => Target::Stdout,
        "stderr" => Target::Stderr,
        _ => bail!("unknown target: {}", s),
    })
}

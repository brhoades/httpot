#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate num_derive;

pub mod prelude {
    pub use anyhow::{anyhow, bail, Error, Result};
    pub use log::{debug, error, info, trace, warn};
}

pub mod fs;
pub mod honeypot;
pub mod http;
pub mod util;

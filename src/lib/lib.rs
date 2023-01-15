#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate num_derive;

pub mod prelude {
    pub use anyhow::{anyhow, bail, Error, Result};
    pub use log::{debug, error, info, trace, warn};
}

pub mod http;
// pub mod test;
pub mod util;

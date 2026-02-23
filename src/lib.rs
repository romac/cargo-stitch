use std::env;

use terrors::OneOf;

mod error;
mod fs;
mod stitch;
mod subcommand;
mod wrapper;

pub use error::{CargoFailed, IoError, PatchFailed, AstGrepFailed};

pub const WRAPPER_ENV: &str = "__CARGO_STITCH_WRAP";

pub fn run() -> Result<(), OneOf<(IoError, PatchFailed, AstGrepFailed, CargoFailed)>> {
    if env::var_os(WRAPPER_ENV).is_some() {
        wrapper::run_wrapper().map_err(OneOf::broaden)
    } else {
        subcommand::run_subcommand().map_err(OneOf::broaden)
    }
}

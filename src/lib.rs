use std::env;

use terrors::OneOf;

#[cfg(not(unix))]
compile_error!("cargo-stitch only supports Unix platforms (Linux, macOS, BSD)");

mod error;
mod fs;
mod stitch;
mod subcommand;
mod wrapper;

pub use error::{AstGrepFailed, CargoFailed, IoError, MissingEnvVar, PatchFailed};

pub const WRAPPER_ENV: &str = "__CARGO_STITCH_WRAP";

pub type Error = OneOf<(
    IoError,
    PatchFailed,
    AstGrepFailed,
    CargoFailed,
    MissingEnvVar,
)>;

pub fn run() -> Result<(), Error> {
    if env::var_os(WRAPPER_ENV).is_some() {
        wrapper::run_wrapper().map_err(OneOf::broaden)
    } else {
        subcommand::run_subcommand().map_err(OneOf::broaden)
    }
}

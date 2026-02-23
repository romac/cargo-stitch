use std::env;
use std::process::Command;

use terrors::OneOf;

#[cfg(not(unix))]
compile_error!("cargo-stitch only supports Unix platforms (Linux, macOS, BSD)");

mod error;
mod fs;
mod stitch;
mod subcommand;
mod wrapper;

pub use error::{AstGrepFailed, CargoFailed, IoError, MissingEnvVar, MissingTool, PatchFailed};

pub const WRAPPER_ENV: &str = "__CARGO_STITCH_WRAP";

pub type Error = OneOf<(
    IoError,
    PatchFailed,
    AstGrepFailed,
    CargoFailed,
    MissingEnvVar,
    MissingTool,
)>;

fn check_required_tools() -> Result<(), OneOf<(MissingTool,)>> {
    // Check for patch
    if Command::new("patch").arg("--version").output().is_err() {
        return Err(OneOf::new(error::MissingTool("patch")));
    }

    // Check for ast-grep (sg)
    if Command::new("sg").arg("--version").output().is_err() {
        return Err(OneOf::new(error::MissingTool("sg (ast-grep)")));
    }

    Ok(())
}

pub fn run() -> Result<(), Error> {
    check_required_tools().map_err(OneOf::broaden)?;

    if env::var_os(WRAPPER_ENV).is_some() {
        wrapper::run_wrapper().map_err(OneOf::broaden)
    } else {
        subcommand::run_subcommand().map_err(OneOf::broaden)
    }
}

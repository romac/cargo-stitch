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

pub use error::{
    AstGrepFailed, CargoFailed, IoError, MissingEnvVar, MissingStitchSet, MissingTool,
    MissingWorkspaceRoot, PatchFailed,
};

pub const WRAPPER_ENV: &str = "__CARGO_STITCH_WRAP";
pub const WORKSPACE_ROOT_ENV: &str = "__CARGO_STITCH_WORKSPACE_ROOT";
pub const STITCH_MANIFEST_ENV: &str = "__CARGO_STITCH_MANIFEST";

pub type Error = OneOf<(
    IoError,
    PatchFailed,
    AstGrepFailed,
    CargoFailed,
    MissingEnvVar,
    MissingTool,
    MissingWorkspaceRoot,
    MissingStitchSet,
)>;

pub(crate) fn check_required_tools(
    need_patch: bool,
    need_sg: bool,
) -> Result<(), OneOf<(MissingTool,)>> {
    if need_patch && Command::new("patch").arg("--version").output().is_err() {
        return Err(OneOf::new(error::MissingTool("patch")));
    }

    if need_sg && Command::new("ast-grep").arg("--version").output().is_err() {
        return Err(OneOf::new(error::MissingTool("ast-grep")));
    }

    Ok(())
}

/// Run the cargo-stitch process
/// # Errors
/// Returns an error if a required tool (like `patch` or `ast-grep`) is missing,
/// or if an underlying cargo build or patch operation fails.
pub fn run() -> Result<(), Error> {
    if env::var_os(WRAPPER_ENV).is_some() {
        wrapper::run_wrapper().map_err(OneOf::broaden)
    } else {
        subcommand::run_subcommand().map_err(OneOf::broaden)
    }
}

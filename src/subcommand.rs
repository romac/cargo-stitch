use std::env;
use std::process::Command;

use terrors::OneOf;

use crate::WRAPPER_ENV;
use crate::error::{CargoFailed, IoError};

pub fn run_subcommand() -> Result<(), OneOf<(IoError, CargoFailed)>> {
    let args: Vec<String> = env::args().collect();

    // cargo stitch build --release
    // argv = ["cargo-stitch", "stitch", "build", "--release"]
    let cargo_args: &[String] = if args.get(1).is_some_and(|a| a == "stitch") {
        &args[2..]
    } else {
        &args[1..]
    };

    let self_exe = env::current_exe().map_err(|e| OneOf::new(IoError(e)))?;

    let status = Command::new("cargo")
        .args(cargo_args)
        .env("RUSTC_WORKSPACE_WRAPPER", &self_exe)
        .env(WRAPPER_ENV, "1")
        .status()
        .map_err(|e| OneOf::new(IoError(e)))?;

    if status.success() {
        Ok(())
    } else {
        Err(OneOf::new(CargoFailed(status.code().unwrap_or(1))))
    }
}

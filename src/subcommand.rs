use std::env;
use std::process::Command;

use terrors::OneOf;

use crate::error::{CargoFailed, IoError, MissingWorkspaceRoot};
use crate::fs::find_workspace_root;
use crate::stitch::StitchSet;
use crate::{STITCH_MANIFEST_ENV, WORKSPACE_ROOT_ENV, WRAPPER_ENV};

pub fn run_subcommand() -> Result<(), OneOf<(IoError, CargoFailed, MissingWorkspaceRoot)>> {
    let args: Vec<String> = env::args().collect();

    // cargo stitch build --release
    // argv = ["cargo-stitch", "stitch", "build", "--release"]
    let cargo_args: &[String] = if args.get(1).is_some_and(|a| a == "stitch") {
        &args[2..]
    } else {
        &args[1..]
    };

    let self_exe = env::current_exe().map_err(|e| OneOf::new(IoError(e)))?;

    let cwd = env::current_dir().map_err(|e| OneOf::new(IoError(e)))?;
    let workspace_root =
        find_workspace_root(&cwd).ok_or_else(|| OneOf::new(MissingWorkspaceRoot(cwd.clone())))?;

    let stitches_dir = workspace_root.join("stitches");
    let manifest = StitchSet::discover_all(&stitches_dir).map_err(OneOf::broaden)?;
    let manifest_json =
        serde_json::to_string(&manifest).map_err(|e| OneOf::new(IoError(e.into())))?;

    let status = Command::new("cargo")
        .args(cargo_args)
        .env("RUSTC_WORKSPACE_WRAPPER", &self_exe)
        .env(WRAPPER_ENV, "1")
        .env(WORKSPACE_ROOT_ENV, &workspace_root)
        .env(STITCH_MANIFEST_ENV, &manifest_json)
        .status()
        .map_err(|e| OneOf::new(IoError(e)))?;

    if status.success() {
        Ok(())
    } else {
        Err(OneOf::new(CargoFailed(status.code().unwrap_or(1))))
    }
}

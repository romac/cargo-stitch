use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use terrors::OneOf;

use crate::error::{AstGrepFailed, IoError, MissingEnvVar, PatchFailed};
use crate::fs::{copy_dir_recursive, find_workspace_root};
use crate::stitch::StitchSet;

pub fn run_wrapper() -> Result<(), OneOf<(IoError, PatchFailed, AstGrepFailed, MissingEnvVar)>> {
    let args: Vec<String> = env::args().collect();
    let rustc = &args[1];
    let rustc_args = &args[2..];

    let pkg_name = match env::var("CARGO_PKG_NAME") {
        Ok(name) => name,
        Err(_) => {
            // No package context (e.g. rustc version probe) — just exec rustc
            let err = Command::new(rustc).args(rustc_args).exec();
            return Err(OneOf::new(IoError(err)));
        }
    };

    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => return Err(OneOf::new(MissingEnvVar("CARGO_MANIFEST_DIR"))),
    };

    let workspace_root = match find_workspace_root(&manifest_dir) {
        Some(root) => root,
        None => {
            // Could not determine workspace root — just exec rustc
            let err = Command::new(rustc).args(rustc_args).exec();
            return Err(OneOf::new(IoError(err)));
        }
    };
    let stitches_dir = workspace_root.join("stitches");

    let stitch_set = match StitchSet::discover(&stitches_dir, &pkg_name).map_err(OneOf::broaden)? {
        Some(s) => s,
        None => {
            let err = Command::new(rustc).args(rustc_args).exec();
            return Err(OneOf::new(IoError(err)));
        }
    };

    // Copy source to target/patched-crates/<pkg_name>/
    let patched_dir = workspace_root
        .join("target")
        .join("patched-crates")
        .join(&pkg_name);

    if patched_dir.exists() {
        fs::remove_dir_all(&patched_dir).map_err(|e| OneOf::new(IoError(e)))?;
    }
    copy_dir_recursive(&manifest_dir, &patched_dir).map_err(|e| OneOf::new(IoError(e)))?;

    // Apply stitch files in filename order
    stitch_set.apply(&patched_dir).map_err(OneOf::broaden)?;

    // Rewrite rustc args: replace manifest_dir with patched_dir
    let manifest_dir_str = manifest_dir.to_string_lossy();
    let patched_dir_str = patched_dir.to_string_lossy();
    let rewritten_args: Vec<String> = rustc_args
        .iter()
        .map(|arg| arg.replace(manifest_dir_str.as_ref(), patched_dir_str.as_ref()))
        .collect();

    let err = Command::new(rustc).args(&rewritten_args).exec();
    Err(OneOf::new(IoError(err)))
}

use std::collections::HashMap;
use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use terrors::OneOf;

const PATCHED_CRATES_DIR: &str = "cargo-stitch";

use crate::error::{AstGrepFailed, IoError, MissingEnvVar, PatchFailed};
use crate::fs::copy_dir_recursive;
use crate::stitch::StitchSet;
use crate::{STITCH_MANIFEST_ENV, WORKSPACE_ROOT_ENV};

/// Execute rustc with the given arguments, replacing the current process.
/// This function only returns if exec fails; on success it never returns.
fn exec_rustc(rustc: &str, args: &[String]) -> IoError {
    IoError(Command::new(rustc).args(args).exec())
}

type WrapperError = OneOf<(IoError, PatchFailed, AstGrepFailed, MissingEnvVar)>;

pub fn run_wrapper() -> Result<(), WrapperError> {
    let args: Vec<String> = env::args().collect();
    let rustc = &args[1];
    let rustc_args = &args[2..];

    // No package context (e.g. rustc version probe) — just exec rustc
    let Ok(pkg_name) = env::var("CARGO_PKG_NAME") else {
        return Err(OneOf::new(exec_rustc(rustc, rustc_args)));
    };

    let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
        return Err(OneOf::new(MissingEnvVar("CARGO_MANIFEST_DIR")));
    };
    let manifest_dir = Utf8PathBuf::from(manifest_dir);

    let Ok(workspace_root) = env::var(WORKSPACE_ROOT_ENV) else {
        return Err(OneOf::new(MissingEnvVar(WORKSPACE_ROOT_ENV)));
    };
    let workspace_root = Utf8PathBuf::from(workspace_root);

    let Ok(manifest_json) = env::var(STITCH_MANIFEST_ENV) else {
        return Err(OneOf::new(MissingEnvVar(STITCH_MANIFEST_ENV)));
    };

    let manifest: HashMap<String, StitchSet> =
        serde_json::from_str(&manifest_json).map_err(|e| OneOf::new(IoError(e.into())))?;

    // No stitches for this package — just exec rustc
    let Some(stitch_set) = manifest.get(&pkg_name) else {
        return Err(OneOf::new(exec_rustc(rustc, rustc_args)));
    };

    // Copy source to a per-process temp dir, apply patches there, then atomically
    // rename into the final location.  This avoids races when the same crate is
    // compiled concurrently (e.g. with different feature combinations): both
    // processes produce identical patched output, so whichever rename wins is fine,
    // and the loser simply discards its temp dir.  Any rustc that already has the
    // previous patched files open via inodes keeps working even after the rename.
    let patched_dir = patched_dir(&pkg_name, &workspace_root);
    let temp_dir = temp_patched_dir(&pkg_name, &workspace_root);

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|e| OneOf::new(IoError(e)))?;
    }

    copy_dir_recursive(&manifest_dir, &temp_dir).map_err(|e| OneOf::new(IoError(e)))?;

    // Apply stitch files in filename order
    stitch_set.apply(&temp_dir).map_err(OneOf::broaden)?;

    // Atomically replace the final patched dir.  On Linux, rename(2) fails with
    // ENOTEMPTY if the destination is a non-empty directory, so we remove it first.
    // The tiny window between the remove and the rename is acceptable: another
    // process racing here will also produce an identical result.
    if patched_dir.exists() {
        fs::remove_dir_all(&patched_dir).map_err(|e| OneOf::new(IoError(e)))?;
    }

    if let Err(e) = fs::rename(&temp_dir, &patched_dir) {
        // Another concurrent process beat us to it.  Both produce identical output,
        // so discard our temp dir and use their result.
        let _ = fs::remove_dir_all(&temp_dir);
        if !patched_dir.exists() {
            return Err(OneOf::new(IoError(e)));
        }
    }

    // Rewrite rustc args: replace manifest_dir with patched_dir
    // Cargo may pass either absolute paths or relative paths (from workspace root),
    // so we need to handle both cases.
    let manifest_dir_str = manifest_dir.as_str();
    let patched_dir_str = patched_dir.as_str();

    // Compute the relative path from workspace root to manifest dir for relative path matching.
    // Add a trailing slash to ensure we match path prefixes only (e.g., "config/src/lib.rs"
    // but not just "config" which could be the crate name argument).
    let relative_manifest_prefix = manifest_dir
        .strip_prefix(&workspace_root)
        .ok()
        .map(|p| format!("{p}/"));

    let rewritten_args: Vec<String> = rustc_args
        .iter()
        .map(|arg| {
            // First try absolute path replacement
            let result = arg.replace(manifest_dir_str, patched_dir_str);
            if result != *arg {
                return result;
            }
            // Then try relative path replacement (for workspace member builds).
            // We require a trailing slash in the prefix to avoid matching the bare crate name.
            if let Some(ref rel_prefix) = relative_manifest_prefix
                && arg.starts_with(rel_prefix.as_str())
            {
                return arg.replacen(rel_prefix.trim_end_matches('/'), patched_dir_str, 1);
            }
            arg.clone()
        })
        .collect();

    Err(OneOf::new(exec_rustc(rustc, &rewritten_args)))
}

fn patched_dir(pkg_name: &str, workspace_root: &Utf8Path) -> Utf8PathBuf {
    workspace_root
        .join("target")
        .join(PATCHED_CRATES_DIR)
        .join(pkg_name)
}

/// A per-process temporary directory used while building the patched source.
/// Named with a leading dot and the process ID to avoid colliding with the
/// final `patched_dir` and with other concurrent compilations of the same crate.
fn temp_patched_dir(pkg_name: &str, workspace_root: &Utf8Path) -> Utf8PathBuf {
    workspace_root
        .join("target")
        .join(PATCHED_CRATES_DIR)
        .join(format!(".{pkg_name}.{}", std::process::id()))
}

use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use terrors::OneOf;

const WRAPPER_ENV: &str = "__CARGO_STITCH_WRAP";

fn main() {
    if let Err(e) = run() {
        eprintln!("cargo-stitch: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), OneOf<(IoError, PatchFailed, CargoFailed)>> {
    if env::var_os(WRAPPER_ENV).is_some() {
        run_wrapper().map_err(OneOf::broaden)
    } else {
        run_subcommand().map_err(OneOf::broaden)
    }
}

struct IoError(std::io::Error);

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct PatchFailed(PathBuf);

impl std::fmt::Display for PatchFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to apply patch: {}", self.0.display())
    }
}

struct CargoFailed(i32);

impl std::fmt::Display for CargoFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cargo exited with status {}", self.0)
    }
}

fn run_subcommand() -> Result<(), OneOf<(IoError, CargoFailed)>> {
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

fn run_wrapper() -> Result<(), OneOf<(IoError, PatchFailed)>> {
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

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = PathBuf::from(&manifest_dir);

    let workspace_root = find_workspace_root(&manifest_dir);
    let stitches_dir = workspace_root.join("stitches").join(&pkg_name);

    if !stitches_dir.is_dir() {
        let err = Command::new(rustc).args(rustc_args).exec();
        return Err(OneOf::new(IoError(err)));
    }

    // Collect and sort patch files
    let mut patches: Vec<PathBuf> = fs::read_dir(&stitches_dir)
        .map_err(|e| OneOf::new(IoError(e)))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "patch") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    patches.sort();

    if patches.is_empty() {
        let err = Command::new(rustc).args(rustc_args).exec();
        return Err(OneOf::new(IoError(err)));
    }

    // Copy source to target/patched-crates/<pkg_name>/
    let patched_dir = workspace_root
        .join("target")
        .join("patched-crates")
        .join(&pkg_name);

    if patched_dir.exists() {
        fs::remove_dir_all(&patched_dir).map_err(|e| OneOf::new(IoError(e)))?;
    }
    copy_dir_recursive(&manifest_dir, &patched_dir).map_err(|e| OneOf::new(IoError(e)))?;

    // Apply patches
    apply_patches(&patched_dir, &patches)?;

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

fn find_workspace_root(manifest_dir: &Path) -> PathBuf {
    let mut root = manifest_dir.to_path_buf();
    let mut current = manifest_dir.to_path_buf();
    while let Some(parent) = current.parent() {
        if parent.join("Cargo.toml").exists() {
            root = parent.to_path_buf();
        } else {
            break;
        }
        current = parent.to_path_buf();
    }
    root
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn apply_patches(dir: &Path, patches: &[PathBuf]) -> Result<(), OneOf<(IoError, PatchFailed)>> {
    for patch in patches {
        let status = Command::new("patch")
            .args(["-s", "-p1"])
            .arg("-i")
            .arg(patch)
            .arg("-d")
            .arg(dir)
            .status()
            .map_err(|e| OneOf::new(IoError(e)))?;

        if !status.success() {
            return Err(OneOf::new(PatchFailed(patch.clone())));
        }
    }
    Ok(())
}

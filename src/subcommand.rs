use std::env;
use std::fs;
use std::process::Command;

use camino::Utf8PathBuf;
use terrors::OneOf;

use crate::error::{CargoFailed, IoError, MissingStitchSet, MissingTool, MissingWorkspaceRoot};
use crate::fs::find_workspace_root;
use crate::stitch::StitchSet;
use crate::{STITCH_MANIFEST_ENV, WORKSPACE_ROOT_ENV, WRAPPER_ENV, check_required_tools};

/// FNV-1a 64-bit hash of `data`.
fn fnv1a_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

type SubcommandError = OneOf<(
    IoError,
    CargoFailed,
    MissingWorkspaceRoot,
    MissingStitchSet,
    MissingTool,
)>;

struct CargoStitchArgs {
    set_name: String,
    set_explicit: bool,
    cargo_args: Vec<String>,
}

impl CargoStitchArgs {
    fn from_env() -> Self {
        let args: Vec<String> = env::args().collect();

        // cargo stitch build --release
        // argv = ["cargo-stitch", "stitch", "build", "--release"]
        let raw_args: &[String] = if args.get(1).is_some_and(|a| a == "stitch") {
            &args[2..]
        } else {
            &args[1..]
        };

        Self::parse(raw_args)
    }

    /// Parse `--set <name>` out of args, returning the set name and the remaining cargo args.
    fn parse(args: &[String]) -> Self {
        let mut set_name = None;
        let mut cargo_args = Vec::new();
        let mut args = args.iter();

        while let Some(arg) = args.next() {
            if arg == "--set" {
                set_name = args.next().cloned();
            } else {
                cargo_args.push(arg.clone());
            }
        }

        Self {
            set_explicit: set_name.is_some(),
            set_name: set_name.unwrap_or_else(|| "default".to_string()),
            cargo_args,
        }
    }
}

pub fn run_subcommand() -> Result<(), SubcommandError> {
    let args = CargoStitchArgs::from_env();

    let self_exe = env::current_exe().map_err(|e| OneOf::new(IoError(e)))?;

    let cwd = Utf8PathBuf::from_path_buf(env::current_dir().map_err(|e| OneOf::new(IoError(e)))?)
        .map_err(|p| {
        OneOf::new(IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("current directory is not valid UTF-8: {}", p.display()),
        )))
    })?;

    let workspace_root =
        find_workspace_root(&cwd).ok_or_else(|| OneOf::new(MissingWorkspaceRoot(cwd.clone())))?;

    let stitches_dir = workspace_root.join("stitches").join(&args.set_name);

    if !stitches_dir.is_dir() && args.set_explicit {
        return Err(OneOf::new(MissingStitchSet(args.set_name)));
    }

    let manifest = StitchSet::discover_all(&stitches_dir).map_err(OneOf::broaden)?;

    let need_patch = manifest.values().any(StitchSet::needs_patch);
    let need_sg = manifest.values().any(StitchSet::needs_sg);
    check_required_tools(need_patch, need_sg).map_err(OneOf::broaden)?;

    let manifest_json =
        serde_json::to_string(&manifest).map_err(|e| OneOf::new(IoError(e.into())))?;

    // Write the manifest to target/cargo-stitch/ using a content hash as the filename.
    // This makes the file content-addressable: same manifest → same file, so concurrent
    // builds with identical manifests converge naturally.  The file is intentionally
    // persistent — cargo clean removes it with the rest of target/.
    // Per the critical invariant: write nothing (and create no directory) when the manifest
    // is empty, so `target/cargo-stitch/` does not exist for crates with no stitch files.
    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .args(&args.cargo_args)
        .env("RUSTC_WORKSPACE_WRAPPER", &self_exe)
        .env(WRAPPER_ENV, "1")
        .env(WORKSPACE_ROOT_ENV, workspace_root.as_str());

    if !manifest.is_empty() {
        let hash = fnv1a_64(manifest_json.as_bytes());
        let stitch_dir = workspace_root.join("target").join("cargo-stitch");
        fs::create_dir_all(&stitch_dir).map_err(|e| OneOf::new(IoError(e)))?;
        let manifest_file = stitch_dir.join(format!(".manifest-{hash:016x}.json"));
        fs::write(&manifest_file, &manifest_json).map_err(|e| OneOf::new(IoError(e)))?;
        cargo_cmd.env(STITCH_MANIFEST_ENV, manifest_file.as_os_str());
    }

    let status = cargo_cmd.status().map_err(|e| OneOf::new(IoError(e)))?;

    if status.success() {
        Ok(())
    } else {
        Err(OneOf::new(CargoFailed(status.code().unwrap_or(1))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_set_defaults() {
        let args = CargoStitchArgs::parse(&["build".to_string(), "--release".to_string()]);
        assert_eq!(args.set_name, "default");
        assert!(!args.set_explicit);
        assert_eq!(args.cargo_args, vec!["build", "--release"]);
    }

    #[test]
    fn parse_with_set() {
        let args = CargoStitchArgs::parse(&[
            "--set".to_string(),
            "custom".to_string(),
            "build".to_string(),
        ]);
        assert_eq!(args.set_name, "custom");
        assert!(args.set_explicit);
        assert_eq!(args.cargo_args, vec!["build"]);
    }

    #[test]
    fn parse_set_at_end() {
        let args = CargoStitchArgs::parse(&[
            "build".to_string(),
            "--release".to_string(),
            "--set".to_string(),
            "myname".to_string(),
        ]);
        assert_eq!(args.set_name, "myname");
        assert!(args.set_explicit);
        assert_eq!(args.cargo_args, vec!["build", "--release"]);
    }

    #[test]
    fn parse_set_without_value_defaults() {
        let args = CargoStitchArgs::parse(&["--set".to_string()]);
        assert_eq!(args.set_name, "default");
        assert!(!args.set_explicit);
    }

    #[test]
    fn fnv1a_64_empty() {
        let h = fnv1a_64(b"");
        assert_eq!(h, 14695981039346656037);
    }

    #[test]
    fn fnv1a_64_known_values() {
        let h1 = fnv1a_64(b"hello");
        let h2 = fnv1a_64(b"world");
        assert_ne!(h1, h2);
        // Verify stability
        assert_eq!(fnv1a_64(b"hello"), h1);
    }

    #[test]
    fn fnv1a_64_different_inputs_differ() {
        let h1 = fnv1a_64(b"abc");
        let h2 = fnv1a_64(b"abd");
        assert_ne!(h1, h2);
    }
}

use std::env;
use std::process::Command;

use camino::Utf8PathBuf;
use terrors::OneOf;

use crate::error::{CargoFailed, IoError, MissingStitchSet, MissingTool, MissingWorkspaceRoot};
use crate::fs::find_workspace_root;
use crate::stitch::StitchSet;
use crate::{STITCH_MANIFEST_ENV, WORKSPACE_ROOT_ENV, WRAPPER_ENV, check_required_tools};

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

    let status = Command::new("cargo")
        .args(&args.cargo_args)
        .env("RUSTC_WORKSPACE_WRAPPER", &self_exe)
        .env(WRAPPER_ENV, "1")
        .env(WORKSPACE_ROOT_ENV, workspace_root.as_str())
        .env(STITCH_MANIFEST_ENV, &manifest_json)
        .status()
        .map_err(|e| OneOf::new(IoError(e)))?;

    if status.success() {
        Ok(())
    } else {
        Err(OneOf::new(CargoFailed(status.code().unwrap_or(1))))
    }
}

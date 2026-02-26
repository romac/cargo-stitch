use std::collections::HashMap;
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use terrors::OneOf;

use crate::error::{AstGrepFailed, IoError, PatchFailed};

/// Print a cargo-style status line to stderr.
///
/// Format: bold yellow `status` right-aligned to 12 characters, followed by the message.
fn cargo_status(status: &str, message: &str) {
    use std::io::Write;

    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "\x1b[1;33m{status:>12}\x1b[0m {message}");
}

#[derive(Serialize, Deserialize)]
pub enum Stitch {
    Patch(Utf8PathBuf),
    SgRule(Utf8PathBuf),
}

impl Stitch {
    pub fn from_path(path: Utf8PathBuf) -> Option<Self> {
        match path.extension() {
            Some("patch") => Some(Stitch::Patch(path)),
            Some("yaml" | "yml") => Some(Stitch::SgRule(path)),
            _ => None,
        }
    }

    pub fn apply(
        &self,
        dir: &Utf8Path,
    ) -> Result<(), OneOf<(IoError, PatchFailed, AstGrepFailed)>> {
        match self {
            Stitch::Patch(file) => {
                let output = Command::new("patch")
                    .args(["-s", "-p1"])
                    .arg("-i")
                    .arg(file.as_str())
                    .arg("-d")
                    .arg(dir.as_str())
                    .output()
                    .map_err(|e| OneOf::new(IoError(e)))?;

                if !output.status.success() {
                    let tool_output = [output.stdout, output.stderr].concat();
                    let output = String::from_utf8_lossy(&tool_output).into_owned();
                    return Err(OneOf::new(PatchFailed {
                        file: file.clone(),
                        output,
                    }));
                }

                let filename = file.file_name().unwrap_or_default();
                cargo_status("Patching", filename);
            }
            Stitch::SgRule(file) => {
                let output = Command::new("sg")
                    .args(["scan", "-r"])
                    .arg(file.as_str())
                    .arg("--update-all")
                    .arg(dir.as_str())
                    .output()
                    .map_err(|e| OneOf::new(IoError(e)))?;

                if !output.status.success() {
                    let tool_output = [output.stdout, output.stderr].concat();
                    let output = String::from_utf8_lossy(&tool_output).into_owned();
                    return Err(OneOf::new(AstGrepFailed {
                        file: file.clone(),
                        output,
                    }));
                }

                // Reformat sg's stderr lines in cargo style
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    if line.starts_with("Applied") {
                        cargo_status("Applied", line.trim_start_matches("Applied").trim());
                    } else if !line.is_empty() {
                        cargo_status("Stitching", line.trim());
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct StitchSet {
    stitches: Vec<Stitch>,
}

impl StitchSet {
    /// Scan all `stitches/*/` subdirectories at once and return a map of `pkg_name` to `StitchSet`.
    pub fn discover_all(
        stitches_dir: &Utf8Path,
    ) -> Result<HashMap<String, StitchSet>, OneOf<(IoError,)>> {
        if !stitches_dir.is_dir() {
            return Ok(HashMap::new());
        }

        let io = |e| OneOf::new(IoError(e));

        let mut pkg_dirs: Vec<_> = stitches_dir
            .read_dir_utf8()
            .map_err(io)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(io)?;

        pkg_dirs.sort_by(|a, b| a.file_name().cmp(b.file_name()));

        pkg_dirs
            .into_iter()
            .filter(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
            .map(|entry| {
                let pkg_name = entry.file_name().to_string();
                let stitches = Self::discover_in(entry.path())?;
                Ok((pkg_name, StitchSet { stitches }))
            })
            .filter(|result| match result {
                Ok((_, set)) => !set.stitches.is_empty(),
                Err(_) => true,
            })
            .collect()
    }

    fn discover_in(dir: &Utf8Path) -> Result<Vec<Stitch>, OneOf<(IoError,)>> {
        let io = |e| OneOf::new(IoError(e));

        let mut paths: Vec<_> = dir
            .read_dir_utf8()
            .map_err(io)?
            .map(|e| e.map(|e| e.path().to_owned()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(io)?;

        paths.sort();

        Ok(paths.into_iter().filter_map(Stitch::from_path).collect())
    }

    pub fn apply(
        &self,
        dir: &Utf8Path,
    ) -> Result<(), OneOf<(IoError, PatchFailed, AstGrepFailed)>> {
        for stitch in &self.stitches {
            stitch.apply(dir)?;
        }
        Ok(())
    }
}

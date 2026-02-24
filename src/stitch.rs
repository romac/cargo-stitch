use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    Patch(PathBuf),
    SgRule(PathBuf),
}

impl Stitch {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("patch") => Some(Stitch::Patch(path)),
            Some("yaml" | "yml") => Some(Stitch::SgRule(path)),
            _ => None,
        }
    }

    pub fn apply(&self, dir: &Path) -> Result<(), OneOf<(IoError, PatchFailed, AstGrepFailed)>> {
        match self {
            Stitch::Patch(file) => {
                let output = Command::new("patch")
                    .args(["-s", "-p1"])
                    .arg("-i")
                    .arg(file)
                    .arg("-d")
                    .arg(dir)
                    .output()
                    .map_err(|e| OneOf::new(IoError(e)))?;

                if !output.status.success() {
                    return Err(OneOf::new(PatchFailed(file.clone())));
                }

                let filename = file.file_name().unwrap_or_default().to_string_lossy();
                cargo_status("Patching", &filename);
            }
            Stitch::SgRule(file) => {
                let output = Command::new("sg")
                    .args(["scan", "-r"])
                    .arg(file)
                    .arg("--update-all")
                    .arg(dir)
                    .output()
                    .map_err(|e| OneOf::new(IoError(e)))?;

                if !output.status.success() {
                    return Err(OneOf::new(AstGrepFailed(file.clone())));
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
        stitches_dir: &Path,
    ) -> Result<HashMap<String, StitchSet>, OneOf<(IoError,)>> {
        let mut manifest = HashMap::new();

        if !stitches_dir.is_dir() {
            return Ok(manifest);
        }

        let mut entries: Vec<_> = fs::read_dir(stitches_dir)
            .map_err(|e| OneOf::new(IoError(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| OneOf::new(IoError(e)))?;

        entries.sort_by_key(std::fs::DirEntry::file_name);

        for entry in entries {
            if !entry
                .file_type()
                .map_err(|e| OneOf::new(IoError(e)))?
                .is_dir()
            {
                continue;
            }

            let pkg_name = entry.file_name().to_string_lossy().into_owned();

            let mut paths: Vec<PathBuf> = Vec::new();
            for file_entry in fs::read_dir(entry.path()).map_err(|e| OneOf::new(IoError(e)))? {
                let file_entry = file_entry.map_err(|e| OneOf::new(IoError(e)))?;
                paths.push(file_entry.path());
            }
            paths.sort();

            let stitches: Vec<Stitch> = paths.into_iter().filter_map(Stitch::from_path).collect();

            if !stitches.is_empty() {
                manifest.insert(pkg_name, StitchSet { stitches });
            }
        }

        Ok(manifest)
    }

    pub fn apply(&self, dir: &Path) -> Result<(), OneOf<(IoError, PatchFailed, AstGrepFailed)>> {
        for stitch in &self.stitches {
            stitch.apply(dir)?;
        }
        Ok(())
    }
}

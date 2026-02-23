use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use terrors::OneOf;

use crate::error::{IoError, PatchFailed, AstGrepFailed};

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
                let status = Command::new("patch")
                    .args(["-s", "-p1"])
                    .arg("-i")
                    .arg(file)
                    .arg("-d")
                    .arg(dir)
                    .status()
                    .map_err(|e| OneOf::new(IoError(e)))?;

                if !status.success() {
                    return Err(OneOf::new(PatchFailed(file.clone())));
                }
            }
            Stitch::SgRule(file) => {
                let status = Command::new("sg")
                    .args(["scan", "-r"])
                    .arg(file)
                    .arg("--update-all")
                    .arg(dir)
                    .status()
                    .map_err(|e| OneOf::new(IoError(e)))?;

                if !status.success() {
                    return Err(OneOf::new(AstGrepFailed(file.clone())));
                }
            }
        }
        Ok(())
    }
}

pub struct StitchSet {
    stitches: Vec<Stitch>,
}

impl StitchSet {
    pub fn discover(stitches_dir: &Path, pkg_name: &str) -> Result<Option<Self>, OneOf<(IoError,)>> {
        let pkg_dir = stitches_dir.join(pkg_name);

        if !pkg_dir.is_dir() {
            return Ok(None);
        }

        let mut paths: Vec<PathBuf> = Vec::new();

        for entry in fs::read_dir(&pkg_dir).map_err(|e| OneOf::new(IoError(e)))? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            paths.push(entry.path());
        }

        paths.sort();

        let stitches: Vec<Stitch> = paths.into_iter().filter_map(Stitch::from_path).collect();

        if stitches.is_empty() {
            return Ok(None);
        }

        Ok(Some(StitchSet { stitches }))
    }

    pub fn apply(&self, dir: &Path) -> Result<(), OneOf<(IoError, PatchFailed, AstGrepFailed)>> {
        for stitch in &self.stitches {
            stitch.apply(dir)?;
        }
        Ok(())
    }
}

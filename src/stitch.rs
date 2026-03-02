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
#[serde(tag = "type", content = "path")]
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

    pub fn path(&self) -> &Utf8Path {
        match self {
            Stitch::Patch(p) | Stitch::SgRule(p) => p.as_path(),
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
                let output = Command::new("ast-grep")
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

                // Reformat ast-grep's stderr lines in cargo style
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

    pub fn file_paths(&self) -> impl Iterator<Item = &Utf8Path> {
        self.stitches.iter().map(|s| s.path())
    }

    pub fn needs_patch(&self) -> bool {
        self.stitches.iter().any(|s| matches!(s, Stitch::Patch(_)))
    }

    pub fn needs_sg(&self) -> bool {
        self.stitches.iter().any(|s| matches!(s, Stitch::SgRule(_)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn stitch_from_path_patch() {
        let s = Stitch::from_path(Utf8PathBuf::from("fix.patch"));
        assert!(matches!(s, Some(Stitch::Patch(_))));
    }

    #[test]
    fn stitch_from_path_yaml() {
        let s = Stitch::from_path(Utf8PathBuf::from("rule.yaml"));
        assert!(matches!(s, Some(Stitch::SgRule(_))));
    }

    #[test]
    fn stitch_from_path_yml() {
        let s = Stitch::from_path(Utf8PathBuf::from("rule.yml"));
        assert!(matches!(s, Some(Stitch::SgRule(_))));
    }

    #[test]
    fn stitch_from_path_txt_returns_none() {
        assert!(Stitch::from_path(Utf8PathBuf::from("readme.txt")).is_none());
    }

    #[test]
    fn stitch_from_path_no_extension_returns_none() {
        assert!(Stitch::from_path(Utf8PathBuf::from("Makefile")).is_none());
    }

    #[test]
    fn stitch_path_returns_inner() {
        let p = Utf8PathBuf::from("stitches/default/crate-a/001.patch");
        let s = Stitch::Patch(p.clone());
        assert_eq!(s.path(), p.as_path());

        let p2 = Utf8PathBuf::from("stitches/default/crate-a/002.yaml");
        let s2 = Stitch::SgRule(p2.clone());
        assert_eq!(s2.path(), p2.as_path());
    }

    #[test]
    fn discover_all_nonexistent_dir() {
        let result =
            StitchSet::discover_all(Utf8Path::new("/nonexistent/stitches/default")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn discover_all_with_stitch_files() {
        let tmp = tempfile::tempdir().unwrap();
        let base = Utf8Path::from_path(tmp.path()).unwrap();
        let stitches_dir = base.join("stitches");

        let pkg_dir = stitches_dir.join("crate-a");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("001.patch"), "").unwrap();
        fs::write(pkg_dir.join("002.yaml"), "").unwrap();

        let result = StitchSet::discover_all(&stitches_dir).unwrap();
        assert!(result.contains_key("crate-a"));
        assert_eq!(result["crate-a"].stitches.len(), 2);
    }

    #[test]
    fn discover_all_empty_subdir_filtered_out() {
        let tmp = tempfile::tempdir().unwrap();
        let base = Utf8Path::from_path(tmp.path()).unwrap();
        let stitches_dir = base.join("stitches");

        let pkg_dir = stitches_dir.join("empty-crate");
        fs::create_dir_all(&pkg_dir).unwrap();
        // No stitch files, just a non-stitch file
        fs::write(pkg_dir.join("readme.txt"), "").unwrap();

        let result = StitchSet::discover_all(&stitches_dir).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn discover_in_returns_sorted_and_filters() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = Utf8Path::from_path(tmp.path()).unwrap();

        fs::write(dir.join("002.yaml"), "").unwrap();
        fs::write(dir.join("001.patch"), "").unwrap();
        fs::write(dir.join("readme.txt"), "").unwrap();

        let stitches = StitchSet::discover_in(dir).unwrap();
        assert_eq!(stitches.len(), 2);
        assert!(matches!(&stitches[0], Stitch::Patch(p) if p.file_name() == Some("001.patch")));
        assert!(matches!(&stitches[1], Stitch::SgRule(p) if p.file_name() == Some("002.yaml")));
    }

    #[test]
    fn needs_patch_and_needs_sg() {
        let set = StitchSet {
            stitches: vec![
                Stitch::Patch(Utf8PathBuf::from("a.patch")),
                Stitch::SgRule(Utf8PathBuf::from("b.yaml")),
            ],
        };
        assert!(set.needs_patch());
        assert!(set.needs_sg());

        let patch_only = StitchSet {
            stitches: vec![Stitch::Patch(Utf8PathBuf::from("a.patch"))],
        };
        assert!(patch_only.needs_patch());
        assert!(!patch_only.needs_sg());

        let sg_only = StitchSet {
            stitches: vec![Stitch::SgRule(Utf8PathBuf::from("b.yml"))],
        };
        assert!(!sg_only.needs_patch());
        assert!(sg_only.needs_sg());

        let empty = StitchSet { stitches: vec![] };
        assert!(!empty.needs_patch());
        assert!(!empty.needs_sg());
    }

    #[test]
    fn file_paths_returns_all() {
        let set = StitchSet {
            stitches: vec![
                Stitch::Patch(Utf8PathBuf::from("a.patch")),
                Stitch::SgRule(Utf8PathBuf::from("b.yaml")),
            ],
        };
        let paths: Vec<_> = set.file_paths().collect();
        assert_eq!(
            paths,
            vec![Utf8Path::new("a.patch"), Utf8Path::new("b.yaml")]
        );
    }

    #[test]
    fn serde_round_trip_stitch() {
        let patch = Stitch::Patch(Utf8PathBuf::from("fix.patch"));
        let json = serde_json::to_string(&patch).unwrap();
        let deser: Stitch = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.path(), Utf8Path::new("fix.patch"));

        let rule = Stitch::SgRule(Utf8PathBuf::from("rule.yml"));
        let json = serde_json::to_string(&rule).unwrap();
        let deser: Stitch = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.path(), Utf8Path::new("rule.yml"));
    }

    #[test]
    fn serde_round_trip_stitch_set() {
        let set = StitchSet {
            stitches: vec![
                Stitch::Patch(Utf8PathBuf::from("a.patch")),
                Stitch::SgRule(Utf8PathBuf::from("b.yaml")),
            ],
        };
        let json = serde_json::to_string(&set).unwrap();
        let deser: StitchSet = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.stitches.len(), 2);
        assert_eq!(deser.stitches[0].path(), Utf8Path::new("a.patch"));
        assert_eq!(deser.stitches[1].path(), Utf8Path::new("b.yaml"));
    }
}

use std::fs;
use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::MetadataCommand;

pub fn find_workspace_root(manifest_dir: &Utf8Path) -> Option<Utf8PathBuf> {
    let metadata = MetadataCommand::new()
        .current_dir(manifest_dir)
        .no_deps()
        .exec()
        .ok()?;

    Some(metadata.workspace_root)
}

pub fn copy_dir_recursive(src: &Utf8Path, dst: &Utf8Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in src.read_dir_utf8()? {
        let entry = entry?;
        let file_name = entry.file_name();

        // Skip target and .git directories to avoid infinite recursion and unnecessary copying
        if file_name == "target" || file_name == ".git" {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(file_name);

        if src_path.is_dir() {
            copy_dir_recursive(src_path, &dst_path)?;
        } else {
            fs::copy(src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Sentinel file written inside `patched_dir` after a successful patch run.
/// Its mtime is used to determine whether re-patching is needed.
const SENTINEL_FILE: &str = ".cargo-stitch";

/// Returns `true` if `patched_dir` was patched more recently than any file in
/// `manifest_dir` (checked recursively) or any of the given `stitch_files`.
///
/// Falls back to `false` on any I/O error to guarantee correctness.
pub fn patched_dir_is_up_to_date(
    patched_dir: &Utf8Path,
    manifest_dir: &Utf8Path,
    stitch_files: &[&Utf8Path],
) -> bool {
    let sentinel = patched_dir.join(SENTINEL_FILE);
    let Ok(meta) = fs::metadata(sentinel.as_std_path()) else {
        return false;
    };
    let Ok(sentinel_mtime) = meta.modified() else {
        return false;
    };

    for &stitch_file in stitch_files {
        if is_newer_than(stitch_file, sentinel_mtime) {
            return false;
        }
    }

    !any_file_newer_than(manifest_dir, sentinel_mtime)
}

/// Write (or overwrite) the sentinel file that records when patching last completed.
pub fn write_sentinel(patched_dir: &Utf8Path) -> std::io::Result<()> {
    fs::write(patched_dir.join(SENTINEL_FILE), b"")
}

fn is_newer_than(path: &Utf8Path, threshold: SystemTime) -> bool {
    fs::metadata(path.as_std_path())
        .and_then(|m| m.modified())
        .is_ok_and(|mtime| mtime > threshold)
}

/// Returns `true` if any entry in `dir` (recursively) has an mtime newer than
/// `threshold`.  Skips `target` and `.git` to mirror `copy_dir_recursive`.
/// Returns `true` on I/O errors to err on the side of re-patching.
fn any_file_newer_than(dir: &Utf8Path, threshold: SystemTime) -> bool {
    let Ok(entries) = dir.read_dir_utf8() else {
        return true;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return true;
        };
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            return true;
        };
        // Directory mtime changes on entry add/remove; file mtime on content change.
        if meta.modified().is_ok_and(|mtime| mtime > threshold) {
            return true;
        }
        if meta.is_dir() && any_file_newer_than(path, threshold) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_dir_recursive_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let src = Utf8Path::from_path(tmp.path()).unwrap().join("src");
        let dst = Utf8Path::from_path(tmp.path()).unwrap().join("dst");

        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.rs"), "fn a() {}").unwrap();
        fs::write(src.join("sub/b.rs"), "fn b() {}").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert_eq!(fs::read_to_string(dst.join("a.rs")).unwrap(), "fn a() {}");
        assert_eq!(
            fs::read_to_string(dst.join("sub/b.rs")).unwrap(),
            "fn b() {}"
        );
    }

    #[test]
    fn copy_dir_recursive_skips_target_and_git() {
        let tmp = tempfile::tempdir().unwrap();
        let src = Utf8Path::from_path(tmp.path()).unwrap().join("src");
        let dst = Utf8Path::from_path(tmp.path()).unwrap().join("dst");

        fs::create_dir_all(src.join("target")).unwrap();
        fs::write(src.join("target/debug"), "binary").unwrap();
        fs::create_dir_all(src.join(".git")).unwrap();
        fs::write(src.join(".git/HEAD"), "ref").unwrap();
        fs::write(src.join("lib.rs"), "code").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("lib.rs").exists());
        assert!(!dst.join("target").exists());
        assert!(!dst.join(".git").exists());
    }

    #[test]
    fn patched_dir_up_to_date_no_sentinel() {
        let tmp = tempfile::tempdir().unwrap();
        let patched = Utf8Path::from_path(tmp.path()).unwrap().join("patched");
        let manifest = Utf8Path::from_path(tmp.path()).unwrap().join("manifest");
        fs::create_dir_all(&patched).unwrap();
        fs::create_dir_all(&manifest).unwrap();

        assert!(!patched_dir_is_up_to_date(&patched, &manifest, &[]));
    }

    #[test]
    fn patched_dir_up_to_date_fresh() {
        let tmp = tempfile::tempdir().unwrap();
        let base = Utf8Path::from_path(tmp.path()).unwrap();
        let patched = base.join("patched");
        let manifest = base.join("manifest");
        fs::create_dir_all(&patched).unwrap();
        fs::create_dir_all(&manifest).unwrap();

        // Create source file first
        fs::write(manifest.join("lib.rs"), "code").unwrap();
        // Small delay to ensure sentinel is newer
        std::thread::sleep(std::time::Duration::from_millis(50));
        // Then write sentinel
        write_sentinel(&patched).unwrap();

        assert!(patched_dir_is_up_to_date(&patched, &manifest, &[]));
    }

    #[test]
    fn patched_dir_stale_stitch_file_newer() {
        let tmp = tempfile::tempdir().unwrap();
        let base = Utf8Path::from_path(tmp.path()).unwrap();
        let patched = base.join("patched");
        let manifest = base.join("manifest");
        fs::create_dir_all(&patched).unwrap();
        fs::create_dir_all(&manifest).unwrap();

        write_sentinel(&patched).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Stitch file written after sentinel
        let stitch = base.join("fix.patch");
        fs::write(&stitch, "patch").unwrap();

        let stitch_ref = Utf8Path::new(stitch.as_str());
        assert!(!patched_dir_is_up_to_date(
            &patched,
            &manifest,
            &[stitch_ref]
        ));
    }

    #[test]
    fn patched_dir_stale_source_file_newer() {
        let tmp = tempfile::tempdir().unwrap();
        let base = Utf8Path::from_path(tmp.path()).unwrap();
        let patched = base.join("patched");
        let manifest = base.join("manifest");
        fs::create_dir_all(&patched).unwrap();
        fs::create_dir_all(&manifest).unwrap();

        write_sentinel(&patched).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Source file written after sentinel
        fs::write(manifest.join("lib.rs"), "new code").unwrap();

        assert!(!patched_dir_is_up_to_date(&patched, &manifest, &[]));
    }

    #[test]
    fn write_sentinel_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = Utf8Path::from_path(tmp.path()).unwrap();
        write_sentinel(dir).unwrap();
        assert!(dir.join(SENTINEL_FILE).exists());
    }
}

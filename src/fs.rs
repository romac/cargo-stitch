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

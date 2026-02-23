use std::fs;
use std::path::{Path, PathBuf};

pub fn find_workspace_root(manifest_dir: &Path) -> PathBuf {
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

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
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

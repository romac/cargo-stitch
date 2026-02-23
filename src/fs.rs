use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;

pub fn find_workspace_root(manifest_dir: &Path) -> Option<PathBuf> {
    let metadata = MetadataCommand::new()
        .current_dir(manifest_dir)
        .no_deps()
        .exec()
        .ok()?;

    Some(metadata.workspace_root.into_std_path_buf())
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

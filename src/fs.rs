use std::fs;

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

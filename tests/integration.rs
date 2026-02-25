use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use cargo_metadata::MetadataCommand;

fn cargo_stitch_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        // Try next to the test executable first (standard layout)
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // remove test binary name
        path.pop(); // remove `deps/`
        path.push("cargo-stitch");
        if path.exists() {
            return path;
        }

        // Fallback: use cargo_metadata to find the target directory, then build
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("failed to get cargo metadata");

        let status = Command::new("cargo")
            .args(["build", "--bin", "cargo-stitch"])
            .status()
            .expect("failed to run cargo build");
        assert!(status.success(), "failed to build cargo-stitch");

        let bin_path = metadata.target_directory.join("debug").join("cargo-stitch");

        bin_path.into_std_path_buf()
    })
}

fn create_workspace(root: &Path) {
    // Workspace Cargo.toml
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crate-a", "crate-b"]
resolver = "2"
"#,
    )
    .unwrap();

    // crate-a: will be patched
    let a = root.join("crate-a");
    fs::create_dir_all(a.join("src")).unwrap();
    fs::write(
        a.join("Cargo.toml"),
        r#"[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(
        a.join("src/lib.rs"),
        r#"pub fn greeting() -> &'static str {
    "hello"
}
"#,
    )
    .unwrap();

    // crate-b: no patches, depends on crate-a
    let b = root.join("crate-b");
    fs::create_dir_all(b.join("src")).unwrap();
    fs::write(
        b.join("Cargo.toml"),
        r#"[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"

[dependencies]
crate-a = { path = "../crate-a" }
"#,
    )
    .unwrap();
    fs::write(
        b.join("src/lib.rs"),
        r#"pub fn message() -> String {
    format!("{} world", crate_a::greeting())
}
"#,
    )
    .unwrap();
}

mod patch {
    use super::*;

    #[test]
    fn build_with_patch() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        create_workspace(root);

        let patch_dir = root.join("stitches/crate-a");
        fs::create_dir_all(&patch_dir).unwrap();
        fs::write(
            patch_dir.join("001-fix.patch"),
            r#"--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 pub fn greeting() -> &'static str {
-    "hello"
+    "patched"
 }
"#,
        )
        .unwrap();

        let output = Command::new(cargo_stitch_bin())
            .args(["stitch", "build"])
            .current_dir(root)
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cargo stitch build failed:\n{stderr}"
        );

        // Verify patched source was created
        let patched_lib = root.join("target/cargo-stitch/crate-a/src/lib.rs");
        assert!(patched_lib.exists(), "patched source should exist");

        let content = fs::read_to_string(&patched_lib).unwrap();
        assert!(
            content.contains("\"patched\""),
            "patched source should contain the patched string, got:\n{content}"
        );
        assert!(
            !content.contains("\"hello\""),
            "patched source should not contain the original string"
        );
    }

    #[test]
    fn build_without_patches() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        create_workspace(root);
        // No patches created

        let output = Command::new(cargo_stitch_bin())
            .args(["stitch", "build"])
            .current_dir(root)
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cargo stitch build failed without patches:\n{stderr}"
        );

        // No cargo-stitch directory should exist
        assert!(
            !root.join("target/cargo-stitch").exists(),
            "target/cargo-stitch dir should not exist when there are no patches"
        );
    }

    #[test]
    fn multiple_patches_applied_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        create_workspace(root);

        let patch_dir = root.join("stitches/crate-a");
        fs::create_dir_all(&patch_dir).unwrap();

        // First patch: change "hello" to "step1"
        fs::write(
            patch_dir.join("001-first.patch"),
            r#"--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 pub fn greeting() -> &'static str {
-    "hello"
+    "step1"
 }
"#,
        )
        .unwrap();

        // Second patch: change "step1" to "step2"
        fs::write(
            patch_dir.join("002-second.patch"),
            r#"--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 pub fn greeting() -> &'static str {
-    "step1"
+    "step2"
 }
"#,
        )
        .unwrap();

        let output = Command::new(cargo_stitch_bin())
            .args(["stitch", "build"])
            .current_dir(root)
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "build failed:\n{stderr}");

        let content =
            fs::read_to_string(root.join("target/cargo-stitch/crate-a/src/lib.rs")).unwrap_or_else(|e| {
                panic!(
                    "patched source should exist after build, but was not found at target/cargo-stitch/crate-a/src/lib.rs: {e}",
                )
            });

        assert!(
            content.contains("\"step2\""),
            "patches should be applied in order, got:\n{content}"
        );
    }
}

mod sg_rule {
    use super::*;

    #[test]
    fn build_with_sg_rule() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        create_workspace(root);

        let rule_dir = root.join("stitches/crate-a");
        fs::create_dir_all(&rule_dir).unwrap();
        fs::write(
            rule_dir.join("001-rename.yaml"),
            r#"id: rename-greeting
language: Rust
rule:
  pattern: '"hello"'
fix: '"rewritten"'
"#,
        )
        .unwrap();

        let output = Command::new(cargo_stitch_bin())
            .args(["stitch", "build"])
            .current_dir(root)
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cargo stitch build failed:\n{stderr}"
        );

        // Verify ast-grep rule was applied
        let patched_lib = root.join("target/cargo-stitch/crate-a/src/lib.rs");
        assert!(patched_lib.exists(), "patched source should exist");

        let content = fs::read_to_string(&patched_lib).unwrap();
        assert!(
            content.contains("\"rewritten\""),
            "ast-grep rule should have rewritten the string, got:\n{content}"
        );
        assert!(
            !content.contains("\"hello\""),
            "original string should not remain after ast-grep rule"
        );
    }

    #[test]
    fn build_with_patch_and_sg_rule() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        create_workspace(root);

        let stitch_dir = root.join("stitches/crate-a");
        fs::create_dir_all(&stitch_dir).unwrap();

        // Patch changes "hello" to "patched"
        fs::write(
            stitch_dir.join("001-fix.patch"),
            r#"--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 pub fn greeting() -> &'static str {
-    "hello"
+    "patched"
 }
"#,
        )
        .unwrap();

        // ast-grep rule changes "patched" to "both"
        // This verifies patches run first, then ast-grep rules
        fs::write(
            stitch_dir.join("002-rename.yaml"),
            r#"id: rename-patched
language: Rust
rule:
  pattern: '"patched"'
fix: '"both"'
"#,
        )
        .unwrap();

        let output = Command::new(cargo_stitch_bin())
            .args(["stitch", "build"])
            .current_dir(root)
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cargo stitch build failed:\n{stderr}"
        );

        let patched_lib = root.join("target/cargo-stitch/crate-a/src/lib.rs");
        let content = fs::read_to_string(&patched_lib).unwrap_or_else(|e| {
            panic!(
                "patched source should exist after build, but was not found at {patched_lib:?}: {e}",
            )
        });

        assert!(
            content.contains("\"both\""),
            "patch should apply first, then ast-grep rule should rewrite, got:\n{content}"
        );
        assert!(
            !content.contains("\"hello\"") && !content.contains("\"patched\""),
            "neither original nor intermediate string should remain, got:\n{content}"
        );
    }
}

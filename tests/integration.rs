use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

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
        // Fallback: build and extract executable path from cargo's JSON output
        let output = Command::new("cargo")
            .args(["build", "--bin", "cargo-stitch", "--message-format=json"])
            .output()
            .unwrap();
        assert!(output.status.success(), "failed to build cargo-stitch");
        let stdout = String::from_utf8(output.stdout).unwrap();
        for line in stdout.lines().rev() {
            // Look for: "executable":"/path/to/cargo-stitch"
            if line.contains("\"executable\"") && line.contains("cargo-stitch") {
                if let Some(start) = line.find("\"executable\":\"") {
                    let rest = &line[start + "\"executable\":\"".len()..];
                    if let Some(end) = rest.find('"') {
                        return PathBuf::from(&rest[..end]);
                    }
                }
            }
        }
        panic!("could not find cargo-stitch binary");
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

fn create_patch(root: &Path) {
    let patch_dir = root.join("stitches/crate-a");
    fs::create_dir_all(&patch_dir).unwrap();

    // Patch that changes "hello" to "patched"
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
}

#[test]
fn build_with_patch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    create_workspace(root);
    create_patch(root);

    let output = Command::new(cargo_stitch_bin())
        .args(["stitch", "build"])
        .current_dir(root)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "cargo stitch build failed:\n{stderr}");

    // Verify patched source was created
    let patched_lib = root.join("target/patched-crates/crate-a/src/lib.rs");
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

    // No patched-crates directory should exist
    assert!(
        !root.join("target/patched-crates").exists(),
        "patched-crates dir should not exist when there are no patches"
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

    let content = fs::read_to_string(root.join("target/patched-crates/crate-a/src/lib.rs")).unwrap();
    assert!(
        content.contains("\"step2\""),
        "patches should be applied in order, got:\n{content}"
    );
}

//! End-to-end test for the example workspace.
//!
//! This test copies the `example/` directory to a temp location and runs
//! `cargo stitch build` and `cargo stitch run` to verify that all patches
//! and ast-grep rules are applied correctly.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use cargo_metadata::MetadataCommand;

/// Get the path to the cargo-stitch binary.
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

/// Get the path to the example/ directory.
fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("example")
}

/// Copy a directory recursively, skipping "target" subdirectories and Cargo.lock.
fn copy_dir_excluding_target(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip target directory and Cargo.lock
        if file_name_str == "target" || file_name_str == "Cargo.lock" {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            copy_dir_excluding_target(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Comprehensive end-to-end test for the example workspace.
///
/// This test verifies:
/// - Patch 001: Error handling (ConfigError, Result return type, Default impl, is_empty)
/// - ast-grep 002: unwrap() → expect("value expected")
/// - Patch 003: Display impl for Value, iter() method, EntriesIter re-export
/// - ast-grep 004: HashMap::new() → HashMap::with_capacity(16)
/// - Runtime: The app compiles and runs correctly with patched code
#[test]
fn example_workspace_builds_and_runs() {
    // === SETUP ===
    let example = example_dir();
    assert!(
        example.exists(),
        "example/ directory should exist at {}",
        example.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path();
    copy_dir_excluding_target(&example, workspace).expect("failed to copy example workspace");

    // === BUILD ===
    let output = Command::new(cargo_stitch_bin())
        .args(["stitch", "build"])
        .current_dir(workspace)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "cargo stitch build failed:\nstderr: {stderr}\nstdout: {stdout}"
    );

    // === VERIFY PATCHED SOURCE EXISTS ===
    let patched_config = workspace.join("target/patched-crates/config");
    assert!(
        patched_config.exists(),
        "patched config crate should exist at {}",
        patched_config.display()
    );

    let patched_lib = patched_config.join("src/lib.rs");
    let patched_parser = patched_config.join("src/parser.rs");
    assert!(
        patched_lib.exists(),
        "patched lib.rs should exist at {}",
        patched_lib.display()
    );
    assert!(
        patched_parser.exists(),
        "patched parser.rs should exist at {}",
        patched_parser.display()
    );

    let lib_content = fs::read_to_string(&patched_lib).unwrap();
    let parser_content = fs::read_to_string(&patched_parser).unwrap();

    // === VERIFY PATCH 001: Error Handling ===

    // New imports
    assert!(
        lib_content.contains("use std::error::Error;"),
        "patch 001: should add Error import\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("use std::io;"),
        "patch 001: should add io import\n\nlib.rs content:\n{lib_content}"
    );

    // ConfigError enum definition
    assert!(
        lib_content.contains("pub enum ConfigError {"),
        "patch 001: should add ConfigError enum\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("Io(io::Error)"),
        "patch 001: ConfigError should have Io variant\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("Parse(parser::ParseError)"),
        "patch 001: ConfigError should have Parse variant\n\nlib.rs content:\n{lib_content}"
    );

    // ConfigError trait implementations
    assert!(
        lib_content.contains("impl std::fmt::Display for ConfigError"),
        "patch 001: should impl Display for ConfigError\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("impl Error for ConfigError"),
        "patch 001: should impl Error for ConfigError\n\nlib.rs content:\n{lib_content}"
    );

    // Default impl for Config
    assert!(
        lib_content.contains("impl Default for Config"),
        "patch 001: should add Default impl for Config\n\nlib.rs content:\n{lib_content}"
    );

    // Load function returns Result
    assert!(
        lib_content.contains("-> Result<Config, ConfigError>"),
        "patch 001: load() should return Result\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        !lib_content.contains("pub fn load(path: &Path) -> Config {"),
        "patch 001: old load() signature should not exist\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains(".map_err(ConfigError::Io)?"),
        "patch 001: should use ? with ConfigError::Io\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains(".map_err(ConfigError::Parse)?"),
        "patch 001: should use ? with ConfigError::Parse\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("Ok(Config { entries })"),
        "patch 001: load() should return Ok(Config)\n\nlib.rs content:\n{lib_content}"
    );

    // is_empty method
    assert!(
        lib_content.contains("fn is_empty(&self) -> bool"),
        "patch 001: should add is_empty() method\n\nlib.rs content:\n{lib_content}"
    );

    // Parser: Clone derive added to ParseError
    assert!(
        parser_content.contains("#[derive(Debug, Clone)]"),
        "patch 001: ParseError should derive Clone\n\nparser.rs content:\n{parser_content}"
    );

    // === VERIFY AST-GREP 002: unwrap() → expect() ===
    assert!(
        lib_content.contains(r#".expect("value expected")"#),
        "ast-grep 002: should convert unwrap() to expect()\n\nlib.rs content:\n{lib_content}"
    );

    // === VERIFY PATCH 003: Display impl and iter() ===

    // Display impl for Value
    assert!(
        lib_content.contains("impl std::fmt::Display for Value"),
        "patch 003: should add Display impl for Value\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("Value::Str(s) => write!(f,"),
        "patch 003: Display should handle Str variant\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("Value::Int(n) => write!(f,"),
        "patch 003: Display should handle Int variant\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("Value::Bool(b) => write!(f,"),
        "patch 003: Display should handle Bool variant\n\nlib.rs content:\n{lib_content}"
    );

    // EntriesIter re-export
    assert!(
        lib_content.contains("pub use std::collections::hash_map::Iter as EntriesIter"),
        "patch 003: should re-export EntriesIter\n\nlib.rs content:\n{lib_content}"
    );

    // iter() method
    assert!(
        lib_content.contains("fn iter(&self)"),
        "patch 003: should add iter() method\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        lib_content.contains("EntriesIter<'_, String, Value>"),
        "patch 003: iter() should return EntriesIter\n\nlib.rs content:\n{lib_content}"
    );

    // === VERIFY AST-GREP 004: HashMap::new() → HashMap::with_capacity(16) ===
    assert!(
        lib_content.contains("HashMap::with_capacity(16)"),
        "ast-grep 004: should use HashMap::with_capacity(16)\n\nlib.rs content:\n{lib_content}"
    );
    assert!(
        !lib_content.contains("HashMap::new()"),
        "ast-grep 004: HashMap::new() should be replaced\n\nlib.rs content:\n{lib_content}"
    );

    // Also check parser.rs for HashMap::new() replacement
    assert!(
        !parser_content.contains("HashMap::new()"),
        "ast-grep 004: HashMap::new() in parser.rs should be replaced\n\nparser.rs content:\n{parser_content}"
    );

    // === RUN THE APP ===
    let output = Command::new(cargo_stitch_bin())
        .args(["stitch", "run"])
        .current_dir(workspace)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "cargo stitch run failed:\nstderr: {stderr}\nstdout: {stdout}"
    );

    // Verify warning about missing config file (goes to stderr)
    assert!(
        stderr.contains("Warning: could not load config:"),
        "app should warn about missing config file\nstderr:\n{stderr}\nstdout:\n{stdout}"
    );

    // Verify app output (goes to stdout)
    assert!(
        stdout.contains("App: my-app"),
        "app output should contain app name\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Port: 8080"),
        "app output should contain port\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Debug: false"),
        "app output should contain debug setting\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Total settings: 3"),
        "app output should show 3 settings\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("All settings:"),
        "app output should show all settings header\nstdout:\n{stdout}"
    );

    // Verify Display impl is working correctly (from patch 003)
    // The iter() loop in main.rs prints "{key} = {value}" using Value's Display impl
    assert!(
        stdout.contains("name = my-app"),
        "app should display name using Value's Display impl\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("port = 8080"),
        "app should display port using Value's Display impl\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("debug = false"),
        "app should display debug using Value's Display impl\nstdout:\n{stdout}"
    );
}

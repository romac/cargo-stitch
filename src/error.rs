use camino::Utf8PathBuf;

#[derive(Debug)]
pub struct IoError(pub std::io::Error);

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct PatchFailed {
    pub file: Utf8PathBuf,
    pub output: String,
}

impl std::fmt::Display for PatchFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to apply patch: {}", self.file)?;
        if !self.output.is_empty() {
            write!(f, "\n{}", self.output.trim_end())?;
        }
        Ok(())
    }
}

pub struct AstGrepFailed {
    pub file: Utf8PathBuf,
    pub output: String,
}

impl std::fmt::Display for AstGrepFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to apply ast-grep rule: {}", self.file)?;
        if !self.output.is_empty() {
            write!(f, "\n{}", self.output.trim_end())?;
        }
        Ok(())
    }
}

pub struct CargoFailed(pub i32);

impl std::fmt::Display for CargoFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cargo exited with status {}", self.0)
    }
}

pub struct MissingEnvVar(pub &'static str);

impl std::fmt::Display for MissingEnvVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing environment variable: {}", self.0)
    }
}

pub struct MissingTool(pub &'static str);

impl std::fmt::Display for MissingTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "required tool not found: {} (is it installed and in PATH?)",
            self.0
        )
    }
}

pub struct MissingWorkspaceRoot(pub Utf8PathBuf);

impl std::fmt::Display for MissingWorkspaceRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "could not find workspace root from manifest directory: {}",
            self.0
        )
    }
}

pub struct MissingStitchSet(pub String);

impl std::fmt::Display for MissingStitchSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stitch set not found: stitches/{}/", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_failed_display_empty_output() {
        let err = PatchFailed {
            file: Utf8PathBuf::from("stitches/default/crate-a/001.patch"),
            output: String::new(),
        };
        assert_eq!(
            err.to_string(),
            "failed to apply patch: stitches/default/crate-a/001.patch"
        );
    }

    #[test]
    fn patch_failed_display_with_output() {
        let err = PatchFailed {
            file: Utf8PathBuf::from("fix.patch"),
            output: "Hunk #1 FAILED\n".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "failed to apply patch: fix.patch\nHunk #1 FAILED"
        );
    }

    #[test]
    fn ast_grep_failed_display_empty_output() {
        let err = AstGrepFailed {
            file: Utf8PathBuf::from("rule.yaml"),
            output: String::new(),
        };
        assert_eq!(err.to_string(), "failed to apply ast-grep rule: rule.yaml");
    }

    #[test]
    fn ast_grep_failed_display_with_output() {
        let err = AstGrepFailed {
            file: Utf8PathBuf::from("rule.yaml"),
            output: "error details\n".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "failed to apply ast-grep rule: rule.yaml\nerror details"
        );
    }

    #[test]
    fn cargo_failed_display() {
        let err = CargoFailed(42);
        assert_eq!(err.to_string(), "cargo exited with status 42");
    }

    #[test]
    fn missing_env_var_display() {
        let err = MissingEnvVar("CARGO_PKG_NAME");
        assert_eq!(
            err.to_string(),
            "missing environment variable: CARGO_PKG_NAME"
        );
    }

    #[test]
    fn missing_tool_display() {
        let err = MissingTool("patch");
        assert_eq!(
            err.to_string(),
            "required tool not found: patch (is it installed and in PATH?)"
        );
    }

    #[test]
    fn missing_workspace_root_display() {
        let err = MissingWorkspaceRoot(Utf8PathBuf::from("/tmp/foo"));
        assert_eq!(
            err.to_string(),
            "could not find workspace root from manifest directory: /tmp/foo"
        );
    }

    #[test]
    fn missing_stitch_set_display() {
        let err = MissingStitchSet("custom".to_string());
        assert_eq!(err.to_string(), "stitch set not found: stitches/custom/");
    }
}

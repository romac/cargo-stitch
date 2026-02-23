use std::path::PathBuf;

pub struct IoError(pub std::io::Error);

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct PatchFailed(pub PathBuf);

impl std::fmt::Display for PatchFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to apply patch: {}", self.0.display())
    }
}

pub struct AstGrepFailed(pub PathBuf);

impl std::fmt::Display for AstGrepFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to apply ast-grep rule: {}", self.0.display())
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Status of the Python environment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PythonStatus {
    /// Python not found on the system
    NotFound,
    /// Python found but version too old (minimum 3.10)
    VersionTooOld { found: String, required: String },
    /// Python found but venv not yet created
    VenvNotCreated {
        python_path: PathBuf,
        python_version: String,
    },
    /// venv exists but some dependencies are missing.
    /// NOTE: Not set by check_status() — used by callers after check_dependencies().
    /// Will be wired up in F8b when uni-converter needs per-script dependency checks.
    DependenciesMissing {
        python_path: PathBuf,
        python_version: String,
        venv_path: PathBuf,
        missing: Vec<String>,
    },
    /// Everything is ready
    Ready {
        python_path: PathBuf,
        python_version: String,
        venv_path: PathBuf,
        venv_python: PathBuf,
    },
}

impl PythonStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, PythonStatus::Ready { .. })
    }

    /// Get the path to the Python executable to use (venv python if ready)
    pub fn python_executable(&self) -> Option<&PathBuf> {
        match self {
            PythonStatus::Ready { venv_python, .. } => Some(venv_python),
            _ => None,
        }
    }
}

/// Python environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonConfig {
    /// Override Python executable path (instead of auto-discovery)
    pub python_path: Option<PathBuf>,
    /// Base directory for venv, scripts, etc. (default: {app_data}/python/)
    pub base_dir: PathBuf,
    /// Minimum Python version (default: "3.10")
    pub min_version: String,
}

impl PythonConfig {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            python_path: None,
            base_dir,
            min_version: "3.10".to_string(),
        }
    }

    pub fn venv_dir(&self) -> PathBuf {
        self.base_dir.join("venv")
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.base_dir.join("scripts")
    }

    pub fn bundled_scripts_dir(&self) -> PathBuf {
        self.scripts_dir().join("bundled")
    }

    pub fn custom_scripts_dir(&self) -> PathBuf {
        self.scripts_dir().join("custom")
    }
}

/// Script execution mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionMode {
    /// Run script, get result, process exits
    OneShot,
    /// Keep process alive, send multiple requests
    LongRunning,
    /// Result comes in chunks
    Streaming,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::OneShot
    }
}

/// Script manifest (parsed from manifest.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptManifest {
    pub script: ScriptInfo,
    pub requirements: Option<RequirementsInfo>,
    pub capabilities: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub entry: String,
    #[serde(default)]
    pub mode: ExecutionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementsInfo {
    /// Path to requirements.txt relative to script directory
    pub file: String,
}

/// Registered script with resolved paths
#[derive(Debug, Clone)]
pub struct RegisteredScript {
    pub manifest: ScriptManifest,
    pub dir: PathBuf,
    pub entry_path: PathBuf,
    pub requirements_path: Option<PathBuf>,
    pub source: ScriptSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptSource {
    Bundled,
    Custom,
}

/// Configuration for a single script execution
#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    /// JSON-RPC method name
    pub method: String,
    /// Parameters (serialized as JSON)
    pub params: serde_json::Value,
    /// Timeout in seconds (0 = no timeout)
    pub timeout_secs: u64,
    /// Optional: extra files to make available in sandbox
    pub input_files: Vec<PathBuf>,
    /// Extra environment variables for the child process
    pub env_vars: Vec<(String, String)>,
}

/// Result of script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// True if script returned a result (not an error)
    pub success: bool,
    /// The result value (if success)
    pub result: Option<serde_json::Value>,
    /// Error message (if not success)
    pub error: Option<String>,
    /// Error code (JSON-RPC error code, if not success)
    pub error_code: Option<i32>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// stderr output (logs)
    pub logs: String,
}

/// Progress notification from a running script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// Percent complete (0-100)
    pub percent: u32,
    /// Human-readable message
    pub message: String,
}

/// Dependency status for a specific requirements file
#[derive(Debug, Clone)]
pub struct DependencyStatus {
    pub installed: Vec<InstalledPackage>,
    pub missing: Vec<String>,
    pub outdated: Vec<OutdatedPackage>,
}

#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct OutdatedPackage {
    pub name: String,
    pub installed: String,
    pub required: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_status_is_ready() {
        let ready = PythonStatus::Ready {
            python_path: PathBuf::from("/usr/bin/python3"),
            python_version: "3.12.0".to_string(),
            venv_path: PathBuf::from("/tmp/venv"),
            venv_python: PathBuf::from("/tmp/venv/bin/python"),
        };
        assert!(ready.is_ready());

        let not_found = PythonStatus::NotFound;
        assert!(!not_found.is_ready());
    }

    #[test]
    fn test_python_config_paths() {
        let config = PythonConfig::new(PathBuf::from("/app/data/python"));
        assert_eq!(config.venv_dir(), PathBuf::from("/app/data/python/venv"));
        assert_eq!(
            config.scripts_dir(),
            PathBuf::from("/app/data/python/scripts")
        );
        assert_eq!(
            config.bundled_scripts_dir(),
            PathBuf::from("/app/data/python/scripts/bundled")
        );
        assert_eq!(
            config.custom_scripts_dir(),
            PathBuf::from("/app/data/python/scripts/custom")
        );
    }

    #[test]
    fn test_execution_mode_default() {
        assert_eq!(ExecutionMode::default(), ExecutionMode::OneShot);
    }

    #[test]
    fn test_script_manifest_parse() {
        let toml_str = r#"
[script]
name = "converter"
version = "1.0.0"
description = "Document converter"
entry = "main.py"
mode = "one-shot"

[requirements]
file = "requirements.txt"

[capabilities]
formats = ["pdf", "docx"]
"#;
        let manifest: ScriptManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.script.name, "converter");
        assert_eq!(manifest.script.version, "1.0.0");
        assert_eq!(manifest.script.mode, ExecutionMode::OneShot);
        assert!(manifest.requirements.is_some());
    }

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult {
            success: true,
            result: Some(serde_json::json!({"markdown": "# Hello"})),
            error: None,
            error_code: None,
            duration_ms: 150,
            logs: String::new(),
        };
        assert!(result.success);
        assert!(result.result.is_some());
    }

    #[test]
    fn test_execution_result_failure() {
        let result = ExecutionResult {
            success: false,
            result: None,
            error: Some("File not found".to_string()),
            error_code: Some(-32602),
            duration_ms: 10,
            logs: "ERROR: No such file\n".to_string(),
        };
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("File not found"));
    }
}

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use tokio::process::Command;

use crate::discovery;
use crate::types::*;

static REQUIREMENT_PKG_RE: OnceLock<regex::Regex> = OnceLock::new();

fn requirement_pkg_regex() -> &'static regex::Regex {
    REQUIREMENT_PKG_RE.get_or_init(|| {
        regex::Regex::new(r"(?i)^\s*([a-z0-9](?:[a-z0-9._-]*[a-z0-9])?|[a-z0-9]+)")
            .expect("valid static regex")
    })
}

/// Extract normalized package name from a requirements.txt line (PEP 508–style name token).
fn package_name_from_requirement_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let cap = requirement_pkg_regex().captures(trimmed)?;
    let name = cap.get(1)?.as_str();
    Some(name.to_lowercase().replace('-', "_"))
}

/// Manages the Python virtual environment.
pub struct PythonEnvironment {
    config: PythonConfig,
}

impl PythonEnvironment {
    pub fn new(config: PythonConfig) -> Self {
        Self { config }
    }

    /// Full health check: discover Python → check venv → check dependencies.
    /// Returns the current status.
    pub async fn check_status(&self) -> PythonStatus {
        // 1. Find Python
        let info = match discovery::discover_python(
            self.config.python_path.as_ref(),
            &self.config.min_version,
        )
        .await
        {
            Some(info) => info,
            None => {
                if let Some(ref path) = self.config.python_path {
                    // User provided path but it didn't work — check if it's version issue
                    if let Some(info) = discovery::discover_python(Some(path), "0.0").await {
                        return PythonStatus::VersionTooOld {
                            found: info.version_string,
                            required: self.config.min_version.clone(),
                        };
                    }
                }
                return PythonStatus::NotFound;
            }
        };

        // 2. Check venv
        let venv_dir = self.config.venv_dir();
        let venv_python = venv_python_path(&venv_dir);

        if !venv_dir.exists() || !venv_python.exists() {
            return PythonStatus::VenvNotCreated {
                python_path: info.path,
                python_version: info.version_string,
            };
        }

        // 3. Check venv python works
        if !check_venv_python(&venv_python).await {
            return PythonStatus::VenvNotCreated {
                python_path: info.path,
                python_version: info.version_string,
            };
        }

        PythonStatus::Ready {
            python_path: info.path,
            python_version: info.version_string,
            venv_path: venv_dir,
            venv_python,
        }
    }

    /// Create the virtual environment.
    pub async fn create_venv(&self) -> Result<PathBuf, uni_common::UniError> {
        let info =
            discovery::discover_python(self.config.python_path.as_ref(), &self.config.min_version)
                .await
                .ok_or_else(|| {
                    uni_common::UniError::Generic(
                        "Python not found. Install Python >= 3.10".to_string(),
                    )
                })?;

        let venv_dir = self.config.venv_dir();

        // Create parent dirs
        if let Some(parent) = venv_dir.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(uni_common::UniError::Io)?;
        }

        // python -m venv {path}
        let output = Command::new(&info.path)
            .args(["-m", "venv", &venv_dir.to_string_lossy()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(uni_common::UniError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(uni_common::UniError::Generic(format!(
                "Failed to create venv: {}",
                stderr
            )));
        }

        // Upgrade pip in venv
        let venv_python = venv_python_path(&venv_dir);
        let _ = Command::new(&venv_python)
            .args(["-m", "pip", "install", "--upgrade", "pip"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await;

        log::info!("Created Python venv at {:?}", venv_dir);
        Ok(venv_dir)
    }

    /// Install dependencies from a requirements.txt file into the venv.
    pub async fn install_requirements(
        &self,
        requirements_path: &Path,
    ) -> Result<(), uni_common::UniError> {
        let venv_python = venv_python_path(&self.config.venv_dir());
        if !venv_python.exists() {
            return Err(uni_common::UniError::Generic(
                "venv not created yet. Call create_venv() first".to_string(),
            ));
        }

        if !requirements_path.exists() {
            return Err(uni_common::UniError::Generic(format!(
                "Requirements file not found: {:?}",
                requirements_path
            )));
        }

        let output = Command::new(&venv_python)
            .args([
                "-m",
                "pip",
                "install",
                "-r",
                &requirements_path.to_string_lossy(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(uni_common::UniError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(uni_common::UniError::Generic(format!(
                "pip install failed: {}",
                stderr
            )));
        }

        log::info!("Installed requirements from {:?}", requirements_path);
        Ok(())
    }

    /// Install a single package (e.g. "pymupdf>=1.24")
    pub async fn install_package(&self, package: &str) -> Result<(), uni_common::UniError> {
        let venv_python = venv_python_path(&self.config.venv_dir());
        if !venv_python.exists() {
            return Err(uni_common::UniError::Generic(
                "venv not created yet".to_string(),
            ));
        }

        let output = Command::new(&venv_python)
            .args(["-m", "pip", "install", package])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(uni_common::UniError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(uni_common::UniError::Generic(format!(
                "pip install {} failed: {}",
                package, stderr
            )));
        }

        Ok(())
    }

    /// List installed packages in venv.
    pub async fn list_packages(&self) -> Result<Vec<InstalledPackage>, uni_common::UniError> {
        let venv_python = venv_python_path(&self.config.venv_dir());
        if !venv_python.exists() {
            return Ok(Vec::new());
        }

        let output = Command::new(&venv_python)
            .args(["-m", "pip", "list", "--format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(uni_common::UniError::Io)?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json = String::from_utf8_lossy(&output.stdout);
        let list: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap_or_default();

        Ok(list
            .into_iter()
            .filter_map(|v| {
                Some(InstalledPackage {
                    name: v.get("name")?.as_str()?.to_string(),
                    version: v.get("version")?.as_str()?.to_string(),
                })
            })
            .collect())
    }

    /// Check which requirements are missing.
    pub async fn check_dependencies(
        &self,
        requirements_path: &Path,
    ) -> Result<DependencyStatus, uni_common::UniError> {
        let installed = self.list_packages().await?;
        let installed_names: std::collections::HashSet<String> = installed
            .iter()
            .map(|p| p.name.to_lowercase().replace('-', "_"))
            .collect();

        let req_content = tokio::fs::read_to_string(requirements_path)
            .await
            .map_err(uni_common::UniError::Io)?;

        let mut missing = Vec::new();
        for line in req_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(pkg_name) = package_name_from_requirement_line(line) else {
                continue;
            };
            if !pkg_name.is_empty() && !installed_names.contains(&pkg_name) {
                missing.push(line.to_string());
            }
        }

        Ok(DependencyStatus {
            installed,
            missing,
            outdated: Vec::new(), // TODO: version comparison
        })
    }

    /// Ensure the environment is fully ready. Creates venv if missing.
    /// Does NOT auto-install dependencies (that's per-script responsibility).
    pub async fn ensure_ready(&self) -> Result<PythonStatus, uni_common::UniError> {
        let status = self.check_status().await;
        match status {
            PythonStatus::Ready { .. } => Ok(status),
            PythonStatus::VenvNotCreated { .. } => {
                self.create_venv().await?;
                Ok(self.check_status().await)
            }
            PythonStatus::NotFound => Err(uni_common::UniError::Generic(
                "Python not found on this system. Please install Python >= 3.10".to_string(),
            )),
            PythonStatus::VersionTooOld { found, required } => Err(uni_common::UniError::Generic(
                format!("Python {} found but {} required", found, required),
            )),
            PythonStatus::DependenciesMissing { .. } => {
                // Venv exists, deps missing — return status as-is, caller decides
                Ok(status)
            }
        }
    }
}

/// Get path to python executable inside venv (platform-specific).
fn venv_python_path(venv_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        venv_dir.join("Scripts").join("python.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        venv_dir.join("bin").join("python")
    }
}

/// Quick check that venv python works.
async fn check_venv_python(python_path: &Path) -> bool {
    Command::new(python_path)
        .args(["-c", "print('ok')"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_venv_python_path() {
        let venv = PathBuf::from("/tmp/test-venv");
        let python = venv_python_path(&venv);
        #[cfg(target_os = "windows")]
        assert_eq!(python, PathBuf::from("/tmp/test-venv/Scripts/python.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(python, PathBuf::from("/tmp/test-venv/bin/python"));
    }

    #[test]
    fn test_python_config_defaults() {
        let config = PythonConfig::new(PathBuf::from("/app/python"));
        assert_eq!(config.min_version, "3.10");
        assert!(config.python_path.is_none());
    }

    // Integration test: only runs if Python is available
    #[tokio::test]
    async fn test_check_status() {
        let tmp = std::env::temp_dir().join("uni-python-test-env");
        let config = PythonConfig::new(tmp.clone());
        let env = PythonEnvironment::new(config);
        let status = env.check_status().await;
        // On systems with Python, should be VenvNotCreated (we didn't create it)
        // On systems without Python, should be NotFound
        // Either way, should NOT be Ready (venv doesn't exist yet)
        assert!(!status.is_ready());
        // Cleanup
        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }
}

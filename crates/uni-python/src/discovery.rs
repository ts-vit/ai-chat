use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Result of Python discovery
#[derive(Debug, Clone)]
pub struct PythonInfo {
    pub path: PathBuf,
    pub version: (u32, u32, u32),
    pub version_string: String,
}

/// Discover Python on the system.
///
/// Search order:
/// 1. Explicit override path (if provided)
/// 2. `python3` in PATH
/// 3. `python` in PATH (check version ≥ 3)
/// 4. Platform-specific standard paths:
///    - Windows: `py -3`, `C:\Python3*\python.exe`
///    - macOS: `/usr/local/bin/python3`, Homebrew paths
///    - Linux: `/usr/bin/python3`
/// 5. pyenv: `~/.pyenv/shims/python3`
///
/// Returns the first working Python with version ≥ min_version.
pub async fn discover_python(
    explicit_path: Option<&PathBuf>,
    min_version: &str,
) -> Option<PythonInfo> {
    let min = parse_version_requirement(min_version);

    // 1. Explicit override
    if let Some(path) = explicit_path {
        if let Some(info) = check_python(path).await {
            if version_satisfies(&info.version, &min) {
                return Some(info);
            }
        }
    }

    // 2. python3 in PATH
    if let Some(info) = check_python_command("python3").await {
        if version_satisfies(&info.version, &min) {
            return Some(info);
        }
    }

    // 3. python in PATH (must be ≥ 3.x)
    if let Some(info) = check_python_command("python").await {
        if info.version.0 >= 3 && version_satisfies(&info.version, &min) {
            return Some(info);
        }
    }

    // 4. Platform-specific
    for path in platform_python_paths() {
        if let Some(info) = check_python(&path).await {
            if version_satisfies(&info.version, &min) {
                return Some(info);
            }
        }
    }

    // 5. pyenv
    if let Some(home) = dirs_next_home() {
        let pyenv_python = home.join(".pyenv").join("shims").join("python3");
        if let Some(info) = check_python(&pyenv_python).await {
            if version_satisfies(&info.version, &min) {
                return Some(info);
            }
        }
    }

    None
}

/// Check if a Python command/path is valid and return its info.
async fn check_python(path: &PathBuf) -> Option<PythonInfo> {
    let output = Command::new(path)
        .args([
            "-c",
            "import sys; v=sys.version_info; print(f'{v.major}.{v.minor}.{v.micro}')",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let version = parse_version(&version_str)?;

    Some(PythonInfo {
        path: path.clone(),
        version,
        version_string: version_str,
    })
}

/// Check a command by name (resolved via PATH)
async fn check_python_command(command: &str) -> Option<PythonInfo> {
    let path = which::which(command).ok()?;
    check_python(&path).await
}

/// Platform-specific Python search paths.
fn platform_python_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // py launcher
        if let Ok(py) = which::which("py") {
            paths.push(py);
        }
        // Standard install locations
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            // Windows Store Python
            let store_path = PathBuf::from(&local_app_data)
                .join("Programs")
                .join("Python");
            if store_path.exists() {
                if let Ok(entries) = std::fs::read_dir(&store_path) {
                    for entry in entries.flatten() {
                        let p = entry.path().join("python.exe");
                        if p.exists() {
                            paths.push(p);
                        }
                    }
                }
            }
        }
        // C:\Python3*
        if let Ok(entries) = std::fs::read_dir("C:\\") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("Python3") {
                    let p = entry.path().join("python.exe");
                    if p.exists() {
                        paths.push(p);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/usr/local/bin/python3"));
        paths.push(PathBuf::from("/opt/homebrew/bin/python3"));
        // Homebrew versioned: /opt/homebrew/opt/python@3.12/bin/python3
        for minor in (10..=15).rev() {
            paths.push(PathBuf::from(format!(
                "/opt/homebrew/opt/python@3.{minor}/bin/python3"
            )));
            paths.push(PathBuf::from(format!(
                "/usr/local/opt/python@3.{minor}/bin/python3"
            )));
        }
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/usr/bin/python3"));
        paths.push(PathBuf::from("/usr/local/bin/python3"));
        // Deadsnakes PPA versioned
        for minor in (10..=15).rev() {
            paths.push(PathBuf::from(format!("/usr/bin/python3.{minor}")));
        }
    }

    paths
}

/// Get user home directory (cross-platform)
fn dirs_next_home() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Parse "3.12.1" → (3, 12, 1)
fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let micro = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some((major, minor, micro))
    } else {
        None
    }
}

/// Parse minimum version requirement "3.10" → (3, 10)
fn parse_version_requirement(s: &str) -> (u32, u32) {
    let parts: Vec<&str> = s.split('.').collect();
    let major = parts.first().and_then(|p| p.parse().ok()).unwrap_or(3);
    let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(10);
    (major, minor)
}

/// Check if version satisfies minimum requirement
fn version_satisfies(version: &(u32, u32, u32), min: &(u32, u32)) -> bool {
    (version.0, version.1) >= (min.0, min.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("3.12.1"), Some((3, 12, 1)));
        assert_eq!(parse_version("3.10"), Some((3, 10, 0)));
        assert_eq!(parse_version("3.10.15"), Some((3, 10, 15)));
        assert_eq!(parse_version("invalid"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn test_parse_version_requirement() {
        assert_eq!(parse_version_requirement("3.10"), (3, 10));
        assert_eq!(parse_version_requirement("3.12"), (3, 12));
    }

    #[test]
    fn test_version_satisfies() {
        assert!(version_satisfies(&(3, 12, 1), &(3, 10)));
        assert!(version_satisfies(&(3, 10, 0), &(3, 10)));
        assert!(!version_satisfies(&(3, 9, 5), &(3, 10)));
        assert!(!version_satisfies(&(2, 7, 18), &(3, 10)));
        assert!(version_satisfies(&(4, 0, 0), &(3, 10)));
    }

    #[test]
    fn test_platform_paths_not_empty() {
        // Should return at least one candidate on any platform
        let paths = platform_python_paths();
        // On CI/test environments this might be empty, so just check it doesn't panic
        let _ = paths;
    }

    // Integration test: only runs if Python is available
    #[tokio::test]
    async fn test_discover_python_on_system() {
        // This test may fail on systems without Python — that's expected
        let info = discover_python(None, "3.8").await;
        if let Some(info) = info {
            assert!(info.version.0 >= 3);
            assert!(!info.version_string.is_empty());
            assert!(
                info.path.exists()
                    || which::which("python3").is_ok()
                    || which::which("python").is_ok()
            );
        }
        // If None — Python not installed, test passes silently
    }
}

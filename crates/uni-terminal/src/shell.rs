/// Auto-detect the best shell for the current platform.
#[cfg(target_os = "windows")]
pub fn detect_shell() -> String {
    if std::process::Command::new("where")
        .arg("pwsh.exe")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "pwsh.exe".to_string();
    }
    if std::process::Command::new("where")
        .arg("powershell.exe")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "powershell.exe".to_string();
    }
    "cmd.exe".to_string()
}

/// Auto-detect the best shell for the current platform.
#[cfg(not(target_os = "windows"))]
pub fn detect_shell() -> String {
    if let Ok(shell) = std::env::var("SHELL") {
        return shell;
    }
    for s in &["/bin/zsh", "/bin/bash", "/bin/sh"] {
        if std::path::Path::new(s).exists() {
            return s.to_string();
        }
    }
    "/bin/sh".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_returns_non_empty() {
        let shell = detect_shell();
        assert!(!shell.is_empty());
    }
}

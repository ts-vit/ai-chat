use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use portable_pty::{CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyDataPayload {
    session_id: String,
    data: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyExitPayload {
    session_id: String,
    code: i32,
}

pub struct TerminalSession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    kill_flag: Arc<AtomicBool>,
}

pub struct TerminalManager {
    sessions: HashMap<String, TerminalSession>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        id: String,
        cols: u16,
        rows: u16,
        app_handle: AppHandle,
        shell: Option<String>,
        proxy_url: Option<String>,
    ) -> Result<(), String> {
        let shell = shell.filter(|s| !s.is_empty()).unwrap_or_else(|| detect_shell());
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system.openpty(size).map_err(|e| e.to_string())?;

        let mut cmd = CommandBuilder::new(&shell);
        if let Some(home) = dirs::home_dir() {
            cmd.cwd(home);
        }

        // Set proxy env vars (SSH tunnel takes priority over manual proxy)
        if let Some(proxy_url) = proxy_url {
            for var in &["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"] {
                cmd.env(var, &proxy_url);
            }
        }

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        // Drop slave after spawn — not needed anymore
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let kill_flag = Arc::new(AtomicBool::new(false));
        let kill_flag_clone = kill_flag.clone();
        let session_id = id.clone();

        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                if kill_flag_clone.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = app_handle.emit(
                            "pty-data",
                            PtyDataPayload {
                                session_id: session_id.clone(),
                                data,
                            },
                        );
                    }
                    Err(_) => break,
                }
            }
            let _ = app_handle.emit(
                "pty-exit",
                PtyExitPayload {
                    session_id,
                    code: 0,
                },
            );
        });

        self.sessions.insert(
            id,
            TerminalSession {
                writer,
                master: pair.master,
                child,
                kill_flag,
            },
        );

        Ok(())
    }

    pub fn write_to_session(&mut self, id: &str, data: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| "Session not found".to_string())?;
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        session.writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn resize_session(&mut self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let session = self
            .sessions
            .get(id)
            .ok_or_else(|| "Session not found".to_string())?;
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    }

    pub fn kill_session(&mut self, id: &str) -> Result<(), String> {
        if let Some(mut session) = self.sessions.remove(id) {
            session.kill_flag.store(true, Ordering::Relaxed);
            let _ = session.child.kill();
        }
        Ok(())
    }

    pub fn kill_all(&mut self) {
        let ids: Vec<String> = self.sessions.keys().cloned().collect();
        for id in ids {
            let _ = self.kill_session(&id);
        }
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        self.kill_all();
    }
}

#[cfg(target_os = "windows")]
fn detect_shell() -> String {
    // Try pwsh.exe first (PowerShell 7+)
    if std::process::Command::new("where")
        .arg("pwsh.exe")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "pwsh.exe".to_string();
    }
    // Fallback to powershell.exe
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

#[cfg(not(target_os = "windows"))]
fn detect_shell() -> String {
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

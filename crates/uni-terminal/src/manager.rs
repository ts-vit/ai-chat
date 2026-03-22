use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use portable_pty::{CommandBuilder, MasterPty, PtySize};

use crate::shell::detect_shell;
use crate::types::{TerminalConfig, TerminalEvent};

struct TerminalSession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    kill_flag: Arc<AtomicBool>,
}

/// Manages multiple PTY terminal sessions.
///
/// Events (data output, exit) are sent via `std::sync::mpsc` channel.
/// The application receives events from the channel and forwards them
/// as needed (e.g. Tauri events).
pub struct TerminalManager {
    sessions: HashMap<String, TerminalSession>,
    event_tx: Sender<TerminalEvent>,
}

impl TerminalManager {
    /// Create a new manager. Returns the manager and a receiver for terminal events.
    pub fn new() -> (Self, std::sync::mpsc::Receiver<TerminalEvent>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (
            Self {
                sessions: HashMap::new(),
                event_tx: tx,
            },
            rx,
        )
    }

    /// Create a new terminal session with the given id and config.
    pub fn create_session(&mut self, id: String, config: TerminalConfig) -> Result<(), String> {
        let shell = config
            .shell
            .filter(|s| !s.is_empty())
            .unwrap_or_else(detect_shell);

        let size = PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system.openpty(size).map_err(|e| e.to_string())?;

        let mut cmd = CommandBuilder::new(&shell);

        let cwd = config.cwd.or_else(dirs::home_dir);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let kill_flag = Arc::new(AtomicBool::new(false));
        let kill_flag_clone = kill_flag.clone();
        let session_id = id.clone();
        let tx = self.event_tx.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                if kill_flag_clone.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = tx.send(TerminalEvent::Data {
                            session_id: session_id.clone(),
                            data,
                        });
                    }
                    Err(_) => break,
                }
            }
            let _ = tx.send(TerminalEvent::Exit {
                session_id,
                code: 0,
            });
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

    /// Write data to a terminal session.
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

    /// Resize a terminal session.
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

    /// Kill a terminal session. Returns Ok even if session doesn't exist.
    pub fn kill_session(&mut self, id: &str) -> Result<(), String> {
        if let Some(mut session) = self.sessions.remove(id) {
            session.kill_flag.store(true, Ordering::Relaxed);
            let _ = session.child.kill();
        }
        Ok(())
    }

    /// Kill all terminal sessions.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_manager_and_receiver() {
        let (manager, _rx) = TerminalManager::new();
        assert!(manager.sessions.is_empty());
    }

    #[test]
    fn kill_nonexistent_session_is_ok() {
        let (mut manager, _rx) = TerminalManager::new();
        assert!(manager.kill_session("nonexistent").is_ok());
    }

    #[test]
    fn write_nonexistent_session_errors() {
        let (mut manager, _rx) = TerminalManager::new();
        assert!(manager.write_to_session("nonexistent", "data").is_err());
    }

    #[test]
    fn resize_nonexistent_session_errors() {
        let (mut manager, _rx) = TerminalManager::new();
        assert!(manager.resize_session("nonexistent", 80, 24).is_err());
    }

    #[test]
    fn kill_all_on_empty_is_noop() {
        let (mut manager, _rx) = TerminalManager::new();
        manager.kill_all();
        assert!(manager.sessions.is_empty());
    }

    #[test]
    #[ignore] // Requires PTY support, may not work in CI
    fn create_and_kill_session() {
        let (mut manager, _rx) = TerminalManager::new();
        let config = TerminalConfig::default();
        let result = manager.create_session("test-1".to_string(), config);
        assert!(result.is_ok());
        assert!(manager.kill_session("test-1").is_ok());
    }
}

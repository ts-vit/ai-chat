use crate::config::ProcessConfig;
use crate::events::{ProcessEvent, ProcessStatus};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{mpsc, watch};
use uni_common::UniError;

pub struct ManagedProcess {
    stdin_tx: Option<mpsc::Sender<Vec<u8>>>,
    event_rx: mpsc::Receiver<ProcessEvent>,
    status_rx: watch::Receiver<ProcessStatus>,
    kill_tx: mpsc::Sender<()>,
    pid: u32,
}

impl ManagedProcess {
    /// Spawn a new managed process.
    pub async fn spawn(config: ProcessConfig) -> Result<Self, UniError> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        if let Some(ref cwd) = config.cwd {
            cmd.current_dir(cwd);
        }

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.stdin(std::process::Stdio::piped());

        #[cfg(windows)]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let mut child = cmd.spawn().map_err(|e| {
            UniError::Generic(format!("Failed to spawn '{}': {}", config.command, e))
        })?;

        let pid = child.id().unwrap_or(0);

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdin = child.stdin.take();

        let (event_tx, event_rx) = mpsc::channel::<ProcessEvent>(256);
        let (status_tx, status_rx) = watch::channel(ProcessStatus::Starting);
        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(64);

        let _ = event_tx.send(ProcessEvent::Started { pid }).await;
        let _ = status_tx.send(ProcessStatus::Running);

        let buffer_size = config.buffer_size;

        // Stdout reader task
        if let Some(stdout) = stdout {
            let tx = event_tx.clone();
            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stdout);
                let mut buf = vec![0u8; buffer_size];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if tx.send(ProcessEvent::Stdout(buf[..n].to_vec())).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx
                                .send(ProcessEvent::Error(format!("stdout read error: {}", e)))
                                .await;
                            break;
                        }
                    }
                }
            });
        }

        // Stderr reader task
        if let Some(stderr) = stderr {
            let tx = event_tx.clone();
            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stderr);
                let mut buf = vec![0u8; buffer_size];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if tx.send(ProcessEvent::Stderr(buf[..n].to_vec())).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx
                                .send(ProcessEvent::Error(format!("stderr read error: {}", e)))
                                .await;
                            break;
                        }
                    }
                }
            });
        }

        // Stdin writer task
        if let Some(mut stdin_handle) = stdin {
            tokio::spawn(async move {
                while let Some(data) = stdin_rx.recv().await {
                    if stdin_handle.write_all(&data).await.is_err() {
                        break;
                    }
                    if stdin_handle.flush().await.is_err() {
                        break;
                    }
                }
            });
        }

        // Wait + kill management task
        // Pass child by value to avoid Arc<Mutex> issues with tokio select
        tokio::spawn(async move {
            tokio::select! {
                status = child.wait() => {
                    match status {
                        Ok(exit_status) => {
                            let code = exit_status.code();
                            let _ = event_tx.send(ProcessEvent::Exited { code }).await;
                            let _ = status_tx.send(ProcessStatus::Exited(code));
                        }
                        Err(e) => {
                            let _ = event_tx.send(ProcessEvent::Error(format!("wait error: {}", e))).await;
                            let _ = status_tx.send(ProcessStatus::Exited(None));
                        }
                    }
                }
                _ = kill_rx.recv() => {
                    // Force kill on all platforms (graceful SIGTERM can be added later)
                    let _ = child.kill().await;
                    let _ = status_tx.send(ProcessStatus::Killed);
                    let _ = event_tx.send(ProcessEvent::Exited { code: None }).await;
                }
            }
        });

        Ok(Self {
            stdin_tx: Some(stdin_tx),
            event_rx,
            status_rx,
            kill_tx,
            pid,
        })
    }

    /// Write raw bytes to stdin.
    pub async fn write_stdin(&self, data: &[u8]) -> Result<(), UniError> {
        if let Some(ref tx) = self.stdin_tx {
            tx.send(data.to_vec())
                .await
                .map_err(|_| UniError::Generic("stdin channel closed".into()))
        } else {
            Err(UniError::Generic("stdin not available".into()))
        }
    }

    /// Write a line (appends newline) to stdin.
    pub async fn write_line(&self, line: &str) -> Result<(), UniError> {
        let mut data = line.as_bytes().to_vec();
        data.push(b'\n');
        self.write_stdin(&data).await
    }

    /// Receive the next event from the process.
    pub async fn next_event(&mut self) -> Option<ProcessEvent> {
        self.event_rx.recv().await
    }

    /// Current process status.
    pub fn status(&self) -> ProcessStatus {
        *self.status_rx.borrow()
    }

    /// Whether the process is still running.
    pub fn is_running(&self) -> bool {
        matches!(
            self.status(),
            ProcessStatus::Running | ProcessStatus::Starting
        )
    }

    /// PID of the spawned process.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Kill the process (force kill).
    pub async fn kill(&self) -> Result<(), UniError> {
        self.kill_tx
            .send(())
            .await
            .map_err(|_| UniError::Generic("kill channel closed".into()))
    }

    /// Alias for kill — force kill the process immediately.
    pub async fn force_kill(&self) -> Result<(), UniError> {
        self.kill().await
    }

    /// Close stdin (sends EOF to the process).
    pub fn close_stdin(&mut self) {
        self.stdin_tx.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProcessConfig;
    use crate::events::ProcessEvent;

    #[tokio::test]
    async fn test_spawn_echo() {
        let config = if cfg!(windows) {
            ProcessConfig::new("cmd").args(["/C", "echo", "hello"])
        } else {
            ProcessConfig::new("echo").arg("hello")
        };

        let mut process = ManagedProcess::spawn(config).await.unwrap();

        let mut got_stdout = false;
        let mut got_exit = false;

        while let Some(event) = process.next_event().await {
            match event {
                ProcessEvent::Stdout(data) => {
                    let text = String::from_utf8_lossy(&data);
                    if text.contains("hello") {
                        got_stdout = true;
                    }
                }
                ProcessEvent::Exited { code } => {
                    assert_eq!(code, Some(0));
                    got_exit = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(got_stdout, "should receive stdout");
        assert!(got_exit, "should receive exit event");
    }

    #[tokio::test]
    async fn test_spawn_with_env() {
        let config = if cfg!(windows) {
            ProcessConfig::new("cmd")
                .args(["/C", "echo", "%TEST_VAR%"])
                .env("TEST_VAR", "uni_test_value")
        } else {
            ProcessConfig::new("sh")
                .args(["-c", "echo $TEST_VAR"])
                .env("TEST_VAR", "uni_test_value")
        };

        let mut process = ManagedProcess::spawn(config).await.unwrap();

        let mut output = String::new();
        while let Some(event) = process.next_event().await {
            match event {
                ProcessEvent::Stdout(data) => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                ProcessEvent::Exited { .. } => break,
                _ => {}
            }
        }

        assert!(output.contains("uni_test_value"));
    }

    #[tokio::test]
    async fn test_spawn_nonexistent() {
        let config = ProcessConfig::new("nonexistent_command_12345");
        let result = ManagedProcess::spawn(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_kill_process() {
        let config = if cfg!(windows) {
            ProcessConfig::new("cmd").args(["/C", "timeout", "/T", "30", "/NOBREAK"])
        } else {
            ProcessConfig::new("sleep").arg("30")
        };

        let mut process = ManagedProcess::spawn(config).await.unwrap();
        assert!(process.is_running());

        process.force_kill().await.unwrap();

        while let Some(event) = process.next_event().await {
            if matches!(event, ProcessEvent::Exited { .. }) {
                break;
            }
        }

        assert!(!process.is_running());
    }

    #[tokio::test]
    async fn test_write_stdin() {
        let config = if cfg!(windows) {
            ProcessConfig::new("findstr").arg(".*")
        } else {
            ProcessConfig::new("cat")
        };

        let mut process = ManagedProcess::spawn(config).await.unwrap();

        process.write_line("hello from stdin").await.unwrap();
        process.close_stdin();

        let mut output = String::new();
        while let Some(event) = process.next_event().await {
            match event {
                ProcessEvent::Stdout(data) => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                ProcessEvent::Exited { .. } => break,
                _ => {}
            }
        }

        assert!(output.contains("hello from stdin"));
    }

    #[tokio::test]
    async fn test_status_transitions() {
        let config = if cfg!(windows) {
            ProcessConfig::new("cmd").args(["/C", "echo", "done"])
        } else {
            ProcessConfig::new("echo").arg("done")
        };

        let mut process = ManagedProcess::spawn(config).await.unwrap();

        while let Some(event) = process.next_event().await {
            if matches!(event, ProcessEvent::Exited { .. }) {
                break;
            }
        }

        assert!(!process.is_running());
    }

    #[tokio::test]
    async fn test_process_pid() {
        let config = if cfg!(windows) {
            ProcessConfig::new("cmd").args(["/C", "echo", "pid test"])
        } else {
            ProcessConfig::new("echo").arg("pid test")
        };

        let process = ManagedProcess::spawn(config).await.unwrap();
        assert!(process.pid() > 0);
    }
}

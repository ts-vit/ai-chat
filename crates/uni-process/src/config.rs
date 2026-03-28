use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for spawning a process.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Command to run.
    pub command: String,
    /// Command-line arguments.
    pub args: Vec<String>,
    /// Working directory (None = inherit current).
    pub cwd: Option<PathBuf>,
    /// Extra environment variables (added to current env).
    pub env: HashMap<String, String>,
    /// Timeout for graceful shutdown before force kill (default: 5s).
    pub kill_timeout: Duration,
    /// Buffer size for reading stdout/stderr (default: 8192).
    pub buffer_size: usize,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            kill_timeout: Duration::from_secs(5),
            buffer_size: 8192,
        }
    }
}

impl ProcessConfig {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            ..Default::default()
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(|a| a.into()));
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn envs(
        mut self,
        envs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.env
            .extend(envs.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    pub fn kill_timeout(mut self, timeout: Duration) -> Self {
        self.kill_timeout = timeout;
        self
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }
}

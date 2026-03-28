/// Event from a child process.
#[derive(Debug, Clone)]
pub enum ProcessEvent {
    /// Process started.
    Started { pid: u32 },
    /// Data from stdout.
    Stdout(Vec<u8>),
    /// Data from stderr.
    Stderr(Vec<u8>),
    /// Process exited.
    Exited { code: Option<i32> },
    /// Error interacting with the process.
    Error(String),
}

/// Process status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    /// Process is starting.
    Starting,
    /// Process is running.
    Running,
    /// Process exited with a code.
    Exited(Option<i32>),
    /// Process was killed.
    Killed,
}

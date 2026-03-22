pub mod manager;
pub mod shell;
pub mod types;

pub use manager::TerminalManager;
pub use shell::detect_shell;
pub use types::{TerminalConfig, TerminalEvent};

pub mod bridge;
pub mod discovery;
pub mod environment;
pub mod executor;
pub mod protocol;
pub mod registry;
pub mod sandbox;
pub mod types;

pub use bridge::ensure_bridge;

// Core types
pub use types::{
    DependencyStatus, ExecutionMode, ExecutionRequest, ExecutionResult, InstalledPackage,
    ProgressInfo, PythonConfig, PythonStatus, RegisteredScript, ScriptInfo, ScriptManifest,
    ScriptSource,
};

// Protocol
pub use protocol::{parse_message, JsonRpcError, JsonRpcRequest, JsonRpcResponse, PythonMessage};

// Discovery
pub use discovery::{discover_python, PythonInfo};

// Environment
pub use environment::PythonEnvironment;

// Executor
pub use executor::PythonExecutor;

// Registry
pub use registry::ScriptRegistry;

// Sandbox
pub use sandbox::Sandbox;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reexports() {
        // Verify key types are accessible from crate root
        let _ = PythonConfig::new(std::path::PathBuf::from("/tmp"));
        let _ = ScriptRegistry::new();
        let _ = PythonStatus::NotFound;
    }
}

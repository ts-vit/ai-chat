use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use uni_common::CancellationToken;

use crate::protocol::{self, JsonRpcRequest, PythonMessage};
use crate::types::*;

/// Executes Python scripts with JSON-RPC protocol.
pub struct PythonExecutor {
    /// Path to Python executable (venv python)
    python_path: PathBuf,
    /// Base dir for sandboxes
    sandbox_base: PathBuf,
}

impl PythonExecutor {
    pub fn new(python_path: PathBuf, sandbox_base: PathBuf) -> Self {
        Self {
            python_path,
            sandbox_base,
        }
    }

    /// Execute a one-shot request against a script.
    ///
    /// 1. Creates sandbox
    /// 2. Copies input files
    /// 3. Spawns Python process
    /// 4. Sends JSON-RPC request via stdin
    /// 5. Reads stdout line by line (responses + progress notifications)
    /// 6. Returns result or error
    /// 7. Cleans up sandbox
    pub async fn execute(
        &self,
        script_path: &Path,
        request: ExecutionRequest,
        cancel_token: CancellationToken,
        on_progress: Option<Box<dyn Fn(ProgressInfo) + Send>>,
    ) -> Result<ExecutionResult, uni_common::UniError> {
        let start = Instant::now();

        // Create sandbox
        let sandbox = crate::sandbox::Sandbox::create(&self.sandbox_base).await?;

        // Copy input files to sandbox
        for input_file in &request.input_files {
            sandbox.add_input_file(input_file).await?;
        }

        // Build params with sandbox paths
        let mut params = request.params.clone();
        if let Some(obj) = params.as_object_mut() {
            obj.insert(
                "_sandbox_dir".to_string(),
                serde_json::Value::String(sandbox.dir.to_string_lossy().to_string()),
            );
            obj.insert(
                "_input_dir".to_string(),
                serde_json::Value::String(sandbox.input_dir.to_string_lossy().to_string()),
            );
            obj.insert(
                "_output_dir".to_string(),
                serde_json::Value::String(sandbox.output_dir.to_string_lossy().to_string()),
            );
        }

        // Spawn Python process
        let mut cmd = Command::new(&self.python_path);
        cmd.arg(script_path);
        for (k, v) in &request.env_vars {
            cmd.env(k, v);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&sandbox.dir)
            .kill_on_drop(true)
            .spawn()
            .map_err(uni_common::UniError::Io)?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| uni_common::UniError::Generic("Failed to open stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| uni_common::UniError::Generic("Failed to open stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| uni_common::UniError::Generic("Failed to open stderr".to_string()))?;

        // Send request
        let rpc_request = JsonRpcRequest::new(1, &request.method, params);
        let request_json = serde_json::to_string(&rpc_request)
            .map_err(|e| uni_common::UniError::Generic(format!("JSON serialize error: {}", e)))?;

        stdin
            .write_all(request_json.as_bytes())
            .await
            .map_err(uni_common::UniError::Io)?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(uni_common::UniError::Io)?;
        stdin.flush().await.map_err(uni_common::UniError::Io)?;
        // Close stdin to signal we're done
        drop(stdin);

        // Read stderr in background
        let stderr_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut logs = String::new();
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                logs.push_str(&line);
                line.clear();
            }
            logs
        });

        // Read stdout lines, parse JSON-RPC messages
        let timeout_secs = if request.timeout_secs > 0 {
            request.timeout_secs
        } else {
            300 // 5 min default
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

        let mut stdout_reader = BufReader::new(stdout);
        let mut result: Option<ExecutionResult> = None;
        let mut line = String::new();

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    let _ = child.kill().await;
                    let logs = stderr_handle.await.unwrap_or_default();
                    sandbox.cleanup().await.ok();
                    return Ok(ExecutionResult {
                        success: false,
                        result: None,
                        error: Some("Cancelled".to_string()),
                        error_code: Some(protocol::error_codes::CANCELLED),
                        duration_ms: start.elapsed().as_millis() as u64,
                        logs,
                    });
                }
                _ = tokio::time::sleep_until(deadline) => {
                    let _ = child.kill().await;
                    let logs = stderr_handle.await.unwrap_or_default();
                    sandbox.cleanup().await.ok();
                    return Ok(ExecutionResult {
                        success: false,
                        result: None,
                        error: Some(format!("Timeout after {}s", timeout_secs)),
                        error_code: Some(protocol::error_codes::TIMEOUT),
                        duration_ms: start.elapsed().as_millis() as u64,
                        logs,
                    });
                }
                bytes_read = stdout_reader.read_line(&mut line) => {
                    match bytes_read {
                        Ok(0) => break, // EOF — process exited
                        Ok(_) => {
                            if let Some(msg) = protocol::parse_message(&line) {
                                match msg {
                                    PythonMessage::Response(resp) => {
                                        if let Some(err) = resp.error {
                                            result = Some(ExecutionResult {
                                                success: false,
                                                result: None,
                                                error: Some(err.message),
                                                error_code: Some(err.code),
                                                duration_ms: start.elapsed().as_millis() as u64,
                                                logs: String::new(), // filled later
                                            });
                                        } else {
                                            result = Some(ExecutionResult {
                                                success: true,
                                                result: resp.result,
                                                error: None,
                                                error_code: None,
                                                duration_ms: start.elapsed().as_millis() as u64,
                                                logs: String::new(),
                                            });
                                        }
                                    }
                                    PythonMessage::Notification(notif) => {
                                        if notif.method == "progress" {
                                            if let Some(params) = notif.params {
                                                if let Ok(progress) = serde_json::from_value::<ProgressInfo>(params) {
                                                    if let Some(ref cb) = on_progress {
                                                        cb(progress);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // else: stray print() line, ignore
                            line.clear();
                        }
                        Err(e) => {
                            log::warn!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        // Wait for process to exit
        let _ = child.wait().await;
        let logs = stderr_handle.await.unwrap_or_default();

        // Fill logs in result
        let exec_result = match result {
            Some(mut r) => {
                r.logs = logs;
                r
            }
            None => ExecutionResult {
                success: false,
                result: None,
                error: Some("Script exited without sending a response".to_string()),
                error_code: Some(protocol::error_codes::INTERNAL_ERROR),
                duration_ms: start.elapsed().as_millis() as u64,
                logs,
            },
        };

        // Cleanup sandbox
        sandbox.cleanup().await.ok();

        Ok(exec_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: find system Python for tests
    async fn find_test_python() -> Option<PathBuf> {
        crate::discovery::discover_python(None, "3.8")
            .await
            .map(|info| info.path)
    }

    /// Helper: write a temp Python script, return its path
    fn write_temp_script(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("uni-python-executor-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[tokio::test]
    async fn test_execute_simple_script() {
        let python = match find_test_python().await {
            Some(p) => p,
            None => return, // Skip if no Python
        };

        let script = write_temp_script(
            "test_simple.py",
            r#"
import sys, json

line = sys.stdin.readline()
req = json.loads(line)
result = {"sum": req["params"]["a"] + req["params"]["b"]}
resp = {"jsonrpc": "2.0", "id": req["id"], "result": result}
print(json.dumps(resp), flush=True)
"#,
        );

        let sandbox_base = std::env::temp_dir().join("uni-python-exec-sandbox");
        let executor = PythonExecutor::new(python, sandbox_base.clone());
        let cancel = CancellationToken::new();

        let result = executor
            .execute(
                &script,
                ExecutionRequest {
                    method: "add".to_string(),
                    params: serde_json::json!({"a": 2, "b": 3}),
                    timeout_secs: 10,
                    input_files: vec![],
                    env_vars: vec![],
                },
                cancel,
                None,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap()["sum"], 5);

        let _ = tokio::fs::remove_dir_all(&sandbox_base).await;
    }

    #[tokio::test]
    async fn test_execute_env_vars() {
        let python = match find_test_python().await {
            Some(p) => p,
            None => return,
        };

        let script = write_temp_script(
            "test_env.py",
            r#"
import os, sys, json

line = sys.stdin.readline()
req = json.loads(line)
val = os.environ.get("UNI_TEST_ENV", "")
resp = {"jsonrpc": "2.0", "id": req["id"], "result": {"env": val}}
print(json.dumps(resp), flush=True)
"#,
        );

        let sandbox_base = std::env::temp_dir().join("uni-python-exec-env");
        let executor = PythonExecutor::new(python, sandbox_base.clone());
        let cancel = CancellationToken::new();

        let result = executor
            .execute(
                &script,
                ExecutionRequest {
                    method: "ping".to_string(),
                    params: serde_json::json!({}),
                    timeout_secs: 10,
                    input_files: vec![],
                    env_vars: vec![("UNI_TEST_ENV".into(), "ok".into())],
                },
                cancel,
                None,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap()["env"], "ok");

        let _ = tokio::fs::remove_dir_all(&sandbox_base).await;
    }

    #[tokio::test]
    async fn test_execute_with_progress() {
        let python = match find_test_python().await {
            Some(p) => p,
            None => return,
        };

        let script = write_temp_script(
            "test_progress.py",
            r#"
import sys, json

line = sys.stdin.readline()
req = json.loads(line)

# Send progress
progress = {"jsonrpc": "2.0", "method": "progress", "params": {"percent": 50, "message": "Halfway..."}}
print(json.dumps(progress), flush=True)

# Send result
resp = {"jsonrpc": "2.0", "id": req["id"], "result": {"status": "done"}}
print(json.dumps(resp), flush=True)
"#,
        );

        let sandbox_base = std::env::temp_dir().join("uni-python-exec-progress");
        let executor = PythonExecutor::new(python, sandbox_base.clone());
        let cancel = CancellationToken::new();

        let progress_received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_clone = progress_received.clone();

        let result = executor
            .execute(
                &script,
                ExecutionRequest {
                    method: "work".to_string(),
                    params: serde_json::json!({}),
                    timeout_secs: 10,
                    input_files: vec![],
                    env_vars: vec![],
                },
                cancel,
                Some(Box::new(move |p| {
                    progress_clone.lock().unwrap().push(p);
                })),
            )
            .await
            .unwrap();

        assert!(result.success);
        let progress = progress_received.lock().unwrap();
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].percent, 50);

        let _ = tokio::fs::remove_dir_all(&sandbox_base).await;
    }

    #[tokio::test]
    async fn test_execute_script_error() {
        let python = match find_test_python().await {
            Some(p) => p,
            None => return,
        };

        let script = write_temp_script(
            "test_error.py",
            r#"
import sys, json

line = sys.stdin.readline()
req = json.loads(line)
resp = {"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32602, "message": "Invalid params"}}
print(json.dumps(resp), flush=True)
"#,
        );

        let sandbox_base = std::env::temp_dir().join("uni-python-exec-error");
        let executor = PythonExecutor::new(python, sandbox_base.clone());
        let cancel = CancellationToken::new();

        let result = executor
            .execute(
                &script,
                ExecutionRequest {
                    method: "fail".to_string(),
                    params: serde_json::json!({}),
                    timeout_secs: 10,
                    input_files: vec![],
                    env_vars: vec![],
                },
                cancel,
                None,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Invalid params"));
        assert_eq!(result.error_code, Some(-32602));

        let _ = tokio::fs::remove_dir_all(&sandbox_base).await;
    }

    #[tokio::test]
    async fn test_execute_cancel() {
        let python = match find_test_python().await {
            Some(p) => p,
            None => return,
        };

        let script = write_temp_script(
            "test_cancel.py",
            r#"
import sys, json, time

line = sys.stdin.readline()
time.sleep(60)  # Sleep long — will be cancelled
"#,
        );

        let sandbox_base = std::env::temp_dir().join("uni-python-exec-cancel");
        let executor = PythonExecutor::new(python, sandbox_base.clone());
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        // Cancel after 100ms
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            cancel_clone.cancel();
        });

        let result = executor
            .execute(
                &script,
                ExecutionRequest {
                    method: "slow".to_string(),
                    params: serde_json::json!({}),
                    timeout_secs: 60,
                    input_files: vec![],
                    env_vars: vec![],
                },
                cancel,
                None,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Cancelled"));

        let _ = tokio::fs::remove_dir_all(&sandbox_base).await;
    }

    #[tokio::test]
    async fn test_execute_crash() {
        let python = match find_test_python().await {
            Some(p) => p,
            None => return,
        };

        // Script crashes without sending response
        let script = write_temp_script(
            "test_crash.py",
            r#"
import sys
line = sys.stdin.readline()
sys.exit(1)
"#,
        );

        let sandbox_base = std::env::temp_dir().join("uni-python-exec-crash");
        let executor = PythonExecutor::new(python, sandbox_base.clone());
        let cancel = CancellationToken::new();

        let result = executor
            .execute(
                &script,
                ExecutionRequest {
                    method: "crash".to_string(),
                    params: serde_json::json!({}),
                    timeout_secs: 10,
                    input_files: vec![],
                    env_vars: vec![],
                },
                cancel,
                None,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap()
            .contains("without sending a response"));

        let _ = tokio::fs::remove_dir_all(&sandbox_base).await;
    }

    #[tokio::test]
    async fn test_execute_stray_prints() {
        let python = match find_test_python().await {
            Some(p) => p,
            None => return,
        };

        // Script prints non-JSON lines before the response
        let script = write_temp_script(
            "test_stray.py",
            r#"
import sys, json

line = sys.stdin.readline()
req = json.loads(line)

# Stray prints (should be ignored by parser)
print("Loading model...", flush=True)
print("Ready!", flush=True)

# Actual response
resp = {"jsonrpc": "2.0", "id": req["id"], "result": {"ok": True}}
print(json.dumps(resp), flush=True)
"#,
        );

        let sandbox_base = std::env::temp_dir().join("uni-python-exec-stray");
        let executor = PythonExecutor::new(python, sandbox_base.clone());
        let cancel = CancellationToken::new();

        let result = executor
            .execute(
                &script,
                ExecutionRequest {
                    method: "test".to_string(),
                    params: serde_json::json!({}),
                    timeout_secs: 10,
                    input_files: vec![],
                    env_vars: vec![],
                },
                cancel,
                None,
            )
            .await
            .unwrap();

        assert!(result.success);

        let _ = tokio::fs::remove_dir_all(&sandbox_base).await;
    }
}

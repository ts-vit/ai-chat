// Built-in Filesystem MCP Server — sandboxed file access for LLM agents
use crate::models::mcp::{FsAuditEntry, FsMcpConfig};
use glob::Pattern;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};

pub struct BuiltinFsServer {
    config: RwLock<FsMcpConfig>,
    canonical_allowed_dirs: RwLock<Vec<PathBuf>>,
    pool: SqlitePool,
    pending_confirmations: Mutex<HashMap<String, Instant>>,
}

impl BuiltinFsServer {
    pub fn new(config: FsMcpConfig, pool: SqlitePool) -> Self {
        let canonical = config
            .allowed_directories
            .iter()
            .filter_map(|d| dunce::canonicalize(d).ok())
            .collect();
        Self {
            config: RwLock::new(config),
            canonical_allowed_dirs: RwLock::new(canonical),
            pool,
            pending_confirmations: Mutex::new(HashMap::new()),
        }
    }

    pub async fn update_config(&self, config: FsMcpConfig) {
        let canonical = config
            .allowed_directories
            .iter()
            .filter_map(|d| dunce::canonicalize(d).ok())
            .collect();
        *self.canonical_allowed_dirs.write().await = canonical;
        *self.config.write().await = config;
    }

    pub async fn handle_jsonrpc(&self, request: Value) -> Value {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "builtin-filesystem", "version": "1.0.0" }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": self.tool_definitions() }
            }),
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or(json!({}));
                let tool_name = params
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
                let result = self.call_tool(tool_name, &arguments).await;
                match result {
                    Ok(content) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": content }],
                            "isError": false
                        }
                    }),
                    Err(err) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": err }],
                            "isError": true
                        }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            }),
        }
    }

    fn tool_definitions(&self) -> Value {
        json!([
            {
                "name": "read_file",
                "description": "Read the complete contents of a file. Only works within allowed directories.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to the file" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "read_multiple_files",
                "description": "Read multiple files simultaneously. Returns content of each file or error for failed reads.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "paths": { "type": "array", "items": { "type": "string" }, "description": "Array of absolute file paths" }
                    },
                    "required": ["paths"]
                }
            },
            {
                "name": "write_file",
                "description": "Write content to a file. Creates parent directories if needed.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to the file" },
                        "content": { "type": "string", "description": "Content to write" }
                    },
                    "required": ["path", "content"]
                }
            },
            {
                "name": "edit_file",
                "description": "Make line-based edits to a file using search/replace pairs.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to the file" },
                        "edits": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "oldText": { "type": "string" },
                                    "newText": { "type": "string" }
                                },
                                "required": ["oldText", "newText"]
                            }
                        },
                        "dryRun": { "type": "boolean", "description": "If true, return diff without modifying file" }
                    },
                    "required": ["path", "edits"]
                }
            },
            {
                "name": "create_directory",
                "description": "Create a directory and all parent directories.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path for new directory" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "list_directory",
                "description": "List contents of a directory with file metadata.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to directory" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "directory_tree",
                "description": "Get a recursive directory tree.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to directory" },
                        "maxDepth": { "type": "integer", "description": "Maximum depth (default 3)" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "move_file",
                "description": "Move or rename a file/directory.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "Source path" },
                        "destination": { "type": "string", "description": "Destination path" }
                    },
                    "required": ["source", "destination"]
                }
            },
            {
                "name": "delete_file",
                "description": "Delete a file.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to file" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "search_files",
                "description": "Search for files matching a glob pattern.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Base directory for search" },
                        "pattern": { "type": "string", "description": "Glob pattern (e.g. '*.rs', '**/*.ts')" },
                        "maxResults": { "type": "integer", "description": "Max results (default 100)" }
                    },
                    "required": ["path", "pattern"]
                }
            },
            {
                "name": "get_file_info",
                "description": "Get metadata about a file or directory.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path" }
                    },
                    "required": ["path"]
                }
            }
        ])
    }

    async fn call_tool(&self, tool_name: &str, arguments: &Value) -> Result<String, String> {
        let chat_id = arguments
            .get("_chatId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let result = match tool_name {
            "read_file" => self.tool_read_file(arguments).await,
            "read_multiple_files" => self.tool_read_multiple_files(arguments).await,
            "write_file" => self.tool_write_file(arguments).await,
            "edit_file" => self.tool_edit_file(arguments).await,
            "create_directory" => self.tool_create_directory(arguments).await,
            "list_directory" => self.tool_list_directory(arguments).await,
            "directory_tree" => self.tool_directory_tree(arguments).await,
            "move_file" => self.tool_move_file(arguments).await,
            "delete_file" => self.tool_delete_file(arguments).await,
            "search_files" => self.tool_search_files(arguments).await,
            "get_file_info" => self.tool_get_file_info(arguments).await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        };

        let path_str = arguments
            .get("path")
            .or(arguments.get("source"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let (result_status, details) = match &result {
            Ok(content) => ("ok".to_string(), content.chars().take(200).collect::<String>()),
            Err(err) => ("error".to_string(), err.clone()),
        };

        self.log_operation(tool_name, &path_str, &result_status, &details, &chat_id)
            .await;

        result
    }

    async fn validate_path(&self, path_str: &str, is_write: bool) -> Result<PathBuf, String> {
        let path = Path::new(path_str);

        // Require absolute path
        if !path.is_absolute() {
            return Err(format!("Path must be absolute: {}", path_str));
        }

        // Check read_only mode
        if is_write {
            let config = self.config.read().await;
            if config.read_only {
                return Err("Read-only mode: write operations are not allowed".to_string());
            }
        }

        // Canonicalize - for non-existing paths, canonicalize the parent
        let canonical = if path.exists() {
            dunce::canonicalize(path).map_err(|e| format!("Cannot resolve path: {}", e))?
        } else {
            let parent = path
                .parent()
                .ok_or_else(|| format!("Invalid path: {}", path_str))?;
            if parent.exists() {
                let canonical_parent = dunce::canonicalize(parent)
                    .map_err(|e| format!("Cannot resolve parent: {}", e))?;
                let file_name = path
                    .file_name()
                    .ok_or_else(|| format!("Invalid filename in path: {}", path_str))?;
                canonical_parent.join(file_name)
            } else {
                // For deeply nested new paths, walk up to find existing ancestor
                let mut existing = parent.to_path_buf();
                let mut remaining = Vec::new();
                loop {
                    if existing.exists() {
                        break;
                    }
                    if let Some(name) = existing.file_name() {
                        remaining.push(name.to_os_string());
                    } else {
                        return Err(format!("No existing ancestor for path: {}", path_str));
                    }
                    existing = existing
                        .parent()
                        .ok_or_else(|| format!("No existing ancestor for path: {}", path_str))?
                        .to_path_buf();
                }
                let mut result = dunce::canonicalize(&existing)
                    .map_err(|e| format!("Cannot resolve ancestor: {}", e))?;
                for component in remaining.into_iter().rev() {
                    result = result.join(component);
                }
                if let Some(file_name) = path.file_name() {
                    result = result.join(file_name);
                }
                result
            }
        };

        // Check against allowed directories
        let allowed_dirs = self.canonical_allowed_dirs.read().await;
        if allowed_dirs.is_empty() {
            return Err("No allowed directories configured".to_string());
        }
        let in_allowed = allowed_dirs.iter().any(|d| canonical.starts_with(d));
        if !in_allowed {
            return Err(format!(
                "Access denied: path is outside allowed directories: {}",
                path_str
            ));
        }

        // If it's a symlink, resolve target and re-check
        if path.is_symlink() {
            let target = std::fs::read_link(path)
                .map_err(|e| format!("Cannot read symlink: {}", e))?;
            let resolved = if target.is_absolute() {
                dunce::canonicalize(&target)
                    .map_err(|e| format!("Cannot resolve symlink target: {}", e))?
            } else {
                let base = path.parent().unwrap_or(Path::new("/"));
                dunce::canonicalize(base.join(&target))
                    .map_err(|e| format!("Cannot resolve symlink target: {}", e))?
            };
            let target_in_allowed = allowed_dirs.iter().any(|d| resolved.starts_with(d));
            if !target_in_allowed {
                return Err(format!(
                    "Access denied: symlink target is outside allowed directories: {}",
                    resolved.display()
                ));
            }
        }

        // Check blocked patterns
        let config = self.config.read().await;
        for component in canonical.components() {
            let component_str = component.as_os_str().to_string_lossy();
            for pattern in &config.blocked_patterns {
                if let Ok(pat) = Pattern::new(pattern) {
                    if pat.matches(&component_str) {
                        return Err(format!(
                            "Access denied: path matches blocked pattern '{}': {}",
                            pattern, path_str
                        ));
                    }
                }
                // Also check exact match
                if &*component_str == pattern {
                    return Err(format!(
                        "Access denied: path matches blocked pattern '{}': {}",
                        pattern, path_str
                    ));
                }
            }
        }

        // Also check the filename itself against blocked patterns
        if let Some(fname) = canonical.file_name() {
            let fname_str = fname.to_string_lossy();
            for pattern in &config.blocked_patterns {
                if let Ok(pat) = Pattern::new(pattern) {
                    if pat.matches(&fname_str) {
                        return Err(format!(
                            "Access denied: filename matches blocked pattern '{}': {}",
                            pattern, path_str
                        ));
                    }
                }
            }
        }

        Ok(canonical)
    }

    async fn check_confirm_destructive(&self, path: &Path) -> Result<(), String> {
        let config = self.config.read().await;
        if !config.confirm_destructive {
            return Ok(());
        }
        if !path.exists() {
            return Ok(()); // New file, no confirmation needed
        }
        let key = path.to_string_lossy().to_string();
        let mut pending = self.pending_confirmations.lock().await;

        if let Some(timestamp) = pending.get(&key) {
            if timestamp.elapsed().as_secs() < 60 {
                pending.remove(&key);
                return Ok(());
            }
        }
        pending.insert(key.clone(), Instant::now());
        Err(format!(
            "Destructive operation on existing file requires confirmation. Call the same tool again within 60 seconds to confirm: {}",
            path.display()
        ))
    }

    async fn log_operation(
        &self,
        tool_name: &str,
        path: &str,
        result: &str,
        details: &str,
        chat_id: &str,
    ) {
        let timestamp = uni_common::now_unix_secs();
        let _ = sqlx::query(
            "INSERT INTO fs_audit_log (timestamp, tool_name, path, result, details, chat_id) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(timestamp)
        .bind(tool_name)
        .bind(path)
        .bind(result)
        .bind(details)
        .bind(chat_id)
        .execute(&self.pool)
        .await;
    }

    // --- Tool implementations ---

    async fn tool_read_file(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let canonical = self.validate_path(path_str, false).await?;

        let config = self.config.read().await;
        let metadata = std::fs::metadata(&canonical)
            .map_err(|e| format!("Cannot read file metadata: {}", e))?;
        if metadata.len() > config.max_file_size_bytes {
            return Err(format!(
                "File too large: {} bytes (max: {} bytes)",
                metadata.len(),
                config.max_file_size_bytes
            ));
        }
        drop(config);

        let bytes =
            std::fs::read(&canonical).map_err(|e| format!("Cannot read file: {}", e))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    async fn tool_read_multiple_files(&self, args: &Value) -> Result<String, String> {
        let paths = args
            .get("paths")
            .and_then(|v| v.as_array())
            .ok_or("Missing 'paths' parameter")?;

        let mut results = Vec::new();
        for p in paths {
            let path_str = p.as_str().unwrap_or("");
            match self
                .tool_read_file(&json!({ "path": path_str }))
                .await
            {
                Ok(content) => results.push(format!("--- {} ---\n{}", path_str, content)),
                Err(e) => results.push(format!("--- {} ---\nERROR: {}", path_str, e)),
            }
        }
        Ok(results.join("\n\n"))
    }

    async fn tool_write_file(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'content' parameter")?;

        let canonical = self.validate_path(path_str, true).await?;
        self.check_confirm_destructive(&canonical).await?;

        if let Some(parent) = canonical.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create parent directories: {}", e))?;
        }
        std::fs::write(&canonical, content)
            .map_err(|e| format!("Cannot write file: {}", e))?;
        Ok(format!("File written: {}", canonical.display()))
    }

    async fn tool_edit_file(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let edits = args
            .get("edits")
            .and_then(|v| v.as_array())
            .ok_or("Missing 'edits' parameter")?;
        let dry_run = args
            .get("dryRun")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let canonical = self.validate_path(path_str, !dry_run).await?;

        let original =
            std::fs::read_to_string(&canonical).map_err(|e| format!("Cannot read file: {}", e))?;
        let mut content = original.clone();

        for edit in edits {
            let old_text = edit
                .get("oldText")
                .and_then(|v| v.as_str())
                .ok_or("Edit missing 'oldText'")?;
            let new_text = edit
                .get("newText")
                .and_then(|v| v.as_str())
                .ok_or("Edit missing 'newText'")?;
            if !content.contains(old_text) {
                return Err(format!("Text not found in file: {:?}", old_text.chars().take(100).collect::<String>()));
            }
            content = content.replacen(old_text, new_text, 1);
        }

        if dry_run {
            // Simple diff output
            let mut diff = String::new();
            for (i, (orig_line, new_line)) in original
                .lines()
                .zip(content.lines())
                .enumerate()
            {
                if orig_line != new_line {
                    diff.push_str(&format!(
                        "Line {}: \n- {}\n+ {}\n",
                        i + 1,
                        orig_line,
                        new_line
                    ));
                }
            }
            if diff.is_empty() {
                diff = "No changes".to_string();
            }
            return Ok(diff);
        }

        if !dry_run {
            self.check_confirm_destructive(&canonical).await?;
        }

        std::fs::write(&canonical, &content)
            .map_err(|e| format!("Cannot write file: {}", e))?;
        Ok(format!(
            "File edited: {} ({} edit(s) applied)",
            canonical.display(),
            edits.len()
        ))
    }

    async fn tool_create_directory(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let canonical = self.validate_path(path_str, true).await?;
        std::fs::create_dir_all(&canonical)
            .map_err(|e| format!("Cannot create directory: {}", e))?;
        Ok(format!("Directory created: {}", canonical.display()))
    }

    async fn tool_list_directory(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let canonical = self.validate_path(path_str, false).await?;

        let entries = std::fs::read_dir(&canonical)
            .map_err(|e| format!("Cannot read directory: {}", e))?;

        let mut lines = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Read entry error: {}", e))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = entry.metadata();
            let (kind, size, mtime) = match meta {
                Ok(m) => {
                    let kind = if m.is_dir() {
                        "dir"
                    } else if m.is_symlink() {
                        "symlink"
                    } else {
                        "file"
                    };
                    let size = m.len();
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    (kind, size, mtime)
                }
                Err(_) => ("unknown", 0, 0),
            };
            lines.push(format!(
                "[{}] {} ({} bytes, modified: {})",
                kind, name, size, mtime
            ));
        }

        if lines.is_empty() {
            Ok("(empty directory)".to_string())
        } else {
            Ok(lines.join("\n"))
        }
    }

    async fn tool_directory_tree(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let max_depth = args
            .get("maxDepth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;

        let canonical = self.validate_path(path_str, false).await?;
        let mut output = String::new();
        self.build_tree(&canonical, "", 0, max_depth, &mut output)
            .await;
        Ok(output)
    }

    async fn build_tree(
        &self,
        dir: &Path,
        prefix: &str,
        depth: usize,
        max_depth: usize,
        output: &mut String,
    ) {
        if depth >= max_depth {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        items.sort_by_key(|e| e.file_name());

        let count = items.len();
        for (i, entry) in items.iter().enumerate() {
            let is_last = i == count - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            output.push_str(&format!(
                "{}{}{}{}\n",
                prefix,
                connector,
                name,
                if is_dir { "/" } else { "" }
            ));
            if is_dir {
                let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
                Box::pin(self.build_tree(&entry.path(), &new_prefix, depth + 1, max_depth, output)).await;
            }
        }
    }

    async fn tool_move_file(&self, args: &Value) -> Result<String, String> {
        let source_str = args
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'source' parameter")?;
        let dest_str = args
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'destination' parameter")?;

        let source = self.validate_path(source_str, true).await?;
        let dest = self.validate_path(dest_str, true).await?;

        if dest.exists() {
            return Err(format!("Destination already exists: {}", dest.display()));
        }

        std::fs::rename(&source, &dest).map_err(|e| format!("Cannot move file: {}", e))?;
        Ok(format!(
            "Moved: {} -> {}",
            source.display(),
            dest.display()
        ))
    }

    async fn tool_delete_file(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let canonical = self.validate_path(path_str, true).await?;
        self.check_confirm_destructive(&canonical).await?;
        std::fs::remove_file(&canonical).map_err(|e| format!("Cannot delete file: {}", e))?;
        Ok(format!("Deleted: {}", canonical.display()))
    }

    async fn tool_search_files(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let pattern_str = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'pattern' parameter")?;
        let max_results = args
            .get("maxResults")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        let canonical = self.validate_path(path_str, false).await?;

        let glob_pattern = format!(
            "{}/{}",
            canonical.display(),
            pattern_str
        );
        let paths = glob::glob(&glob_pattern).map_err(|e| format!("Invalid pattern: {}", e))?;

        let mut results = Vec::new();
        for entry in paths {
            if results.len() >= max_results {
                break;
            }
            if let Ok(p) = entry {
                results.push(p.display().to_string());
            }
        }

        if results.is_empty() {
            Ok("No files found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }

    async fn tool_get_file_info(&self, args: &Value) -> Result<String, String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let canonical = self.validate_path(path_str, false).await?;

        let metadata = std::fs::metadata(&canonical)
            .map_err(|e| format!("Cannot read metadata: {}", e))?;

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_symlink() {
            "symlink"
        } else {
            "file"
        };

        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let created = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(format!(
            "Path: {}\nType: {}\nSize: {} bytes\nModified: {}\nCreated: {}\nReadonly: {}",
            canonical.display(),
            file_type,
            metadata.len(),
            mtime,
            created,
            metadata.permissions().readonly()
        ))
    }
}

// Public helpers for commands
pub async fn get_audit_log(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<FsAuditEntry>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, timestamp, tool_name, path, result, details, chat_id FROM fs_audit_log ORDER BY timestamp DESC LIMIT ? OFFSET ?"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|r| FsAuditEntry {
            id: r.get("id"),
            timestamp: r.get("timestamp"),
            tool_name: r.get("tool_name"),
            path: r.get("path"),
            result: r.get("result"),
            details: r.get::<String, _>("details"),
            chat_id: r.get::<String, _>("chat_id"),
        })
        .collect())
}

pub async fn clear_audit_log(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query("DELETE FROM fs_audit_log")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

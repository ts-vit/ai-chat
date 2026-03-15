// Tauri commands for MCP (connect, disconnect, list, call tool, DB CRUD)
use crate::models::mcp::{DbMcpServer, FsAuditEntry, FsMcpConfig, McpConnectionInfo, McpTool, McpToolInfo, McpToolResult};
use crate::services::builtin_fs_server::{self, BuiltinFsServer};
use crate::services::mcp_manager::McpManager;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

type Pool = sqlx::SqlitePool;

fn row_to_db_mcp_server(row: &sqlx::sqlite::SqliteRow) -> DbMcpServer {
    let args_str: String = row.get("args");
    let env_str: String = row.get("env");
    let enabled_int: i64 = row.get("enabled");
    let server_type: String = row.try_get("server_type").unwrap_or_else(|_| "stdio".to_string());
    let config: String = row.try_get("config").unwrap_or_else(|_| "{}".to_string());
    DbMcpServer {
        id: row.get("id"),
        name: row.get("name"),
        command: row.get("command"),
        args: serde_json::from_str(&args_str).unwrap_or_default(),
        env: serde_json::from_str(&env_str).unwrap_or_default(),
        enabled: enabled_int != 0,
        created_at: row.get("created_at"),
        server_type,
        config,
    }
}

// --- DB CRUD ---

#[tauri::command]
pub async fn mcp_get_servers(pool: State<'_, Pool>) -> Result<Vec<DbMcpServer>, String> {
    let rows = sqlx::query(
        "SELECT id, name, command, args, env, enabled, created_at, server_type, config FROM mcp_servers ORDER BY created_at ASC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_db_mcp_server).collect())
}

#[tauri::command]
pub async fn mcp_add_server(
    pool: State<'_, Pool>,
    id: Option<String>,
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    enabled: bool,
) -> Result<DbMcpServer, String> {
    let server_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let args_json = serde_json::to_string(&args).unwrap_or_else(|_| "[]".into());
    let env_json = serde_json::to_string(&env).unwrap_or_else(|_| "{}".into());
    let enabled_int: i64 = if enabled { 1 } else { 0 };

    sqlx::query(
        "INSERT INTO mcp_servers (id, name, command, args, env, enabled, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&server_id)
    .bind(&name)
    .bind(&command)
    .bind(&args_json)
    .bind(&env_json)
    .bind(enabled_int)
    .bind(created_at)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    log::info!("MCP server added: {} ({})", name, server_id);

    Ok(DbMcpServer {
        id: server_id,
        name,
        command,
        args,
        env,
        enabled,
        created_at,
        server_type: "stdio".to_string(),
        config: "{}".to_string(),
    })
}

#[tauri::command]
pub async fn mcp_update_server(
    pool: State<'_, Pool>,
    manager: State<'_, Arc<McpManager>>,
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    enabled: bool,
) -> Result<(), String> {
    let old_row = sqlx::query("SELECT id, name, command, args, env, enabled, created_at, server_type, config FROM mcp_servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("MCP server not found: {}", id))?;
    let old = row_to_db_mcp_server(&old_row);

    let args_json = serde_json::to_string(&args).unwrap_or_else(|_| "[]".into());
    let env_json = serde_json::to_string(&env).unwrap_or_else(|_| "{}".into());
    let enabled_int: i64 = if enabled { 1 } else { 0 };

    sqlx::query("UPDATE mcp_servers SET name = ?, command = ?, args = ?, env = ?, enabled = ? WHERE id = ?")
        .bind(&name)
        .bind(&command)
        .bind(&args_json)
        .bind(&env_json)
        .bind(enabled_int)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let connections = manager.list_connections().await;
    let is_connected = connections.iter().any(|c| c.id == id);

    if is_connected {
        let params_changed = old.name != name
            || old.command != command
            || old.args != args
            || old.env != env;

        if params_changed {
            log::info!("MCP server {} params changed, reconnecting", id);
            let _ = manager.disconnect(&id).await;
            if let Err(e) = manager.connect(&id, &name, command, args, env).await {
                log::error!("MCP server {} reconnect failed: {}", id, e);
                return Err(format!("Server updated but reconnect failed: {}", e));
            }
        }
    }

    log::debug!("MCP server updated: {}", id);
    Ok(())
}

#[tauri::command]
pub async fn mcp_remove_server(
    pool: State<'_, Pool>,
    manager: State<'_, Arc<McpManager>>,
    id: String,
) -> Result<(), String> {
    let _ = manager.disconnect(&id).await;

    sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    log::info!("MCP server removed: {}", id);
    Ok(())
}

#[tauri::command]
pub async fn mcp_toggle_server(
    pool: State<'_, Pool>,
    manager: State<'_, Arc<McpManager>>,
    fs_server: State<'_, Arc<BuiltinFsServer>>,
    id: String,
    enabled: bool,
) -> Result<DbMcpServer, String> {
    let enabled_int: i64 = if enabled { 1 } else { 0 };

    sqlx::query("UPDATE mcp_servers SET enabled = ? WHERE id = ?")
        .bind(enabled_int)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let row = sqlx::query("SELECT id, name, command, args, env, enabled, created_at, server_type, config FROM mcp_servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("MCP server not found: {}", id))?;
    let server = row_to_db_mcp_server(&row);

    if enabled {
        if server.server_type == "builtin" {
            if let Err(e) = manager
                .connect_builtin(&server.id, &server.name, fs_server.inner().clone())
                .await
            {
                log::error!("Builtin MCP server {} toggle-connect failed: {}", id, e);
                return Err(format!("Server enabled but connect failed: {}", e));
            }
        } else {
            if let Err(e) = manager
                .connect(&server.id, &server.name, server.command.clone(), server.args.clone(), server.env.clone())
                .await
            {
                log::error!("MCP server {} toggle-connect failed: {}", id, e);
                return Err(format!("Server enabled but connect failed: {}", e));
            }
        }
        log::info!("MCP server enabled and connected: {}", id);
    } else {
        let _ = manager.disconnect(&id).await;
        log::info!("MCP server disabled and disconnected: {}", id);
    }

    Ok(server)
}

// --- Connect by DB id ---

#[tauri::command]
pub async fn mcp_connect_by_id(
    pool: State<'_, Pool>,
    manager: State<'_, Arc<McpManager>>,
    fs_server: State<'_, Arc<BuiltinFsServer>>,
    server_id: String,
) -> Result<Vec<McpTool>, String> {
    let row = sqlx::query("SELECT id, name, command, args, env, enabled, created_at, server_type, config FROM mcp_servers WHERE id = ?")
        .bind(&server_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("MCP server not found: {}", server_id))?;
    let server = row_to_db_mcp_server(&row);

    if server.server_type == "builtin" {
        manager
            .connect_builtin(&server.id, &server.name, fs_server.inner().clone())
            .await
    } else {
        manager
            .connect(&server.id, &server.name, server.command, server.args, server.env)
            .await
    }
}

// --- Existing commands (State type updated to Arc<McpManager>) ---

#[tauri::command]
pub async fn mcp_connect(
    server_id: String,
    name: String,
    command: String,
    args: Vec<String>,
    env: Option<HashMap<String, String>>,
    manager: State<'_, Arc<McpManager>>,
) -> Result<Vec<McpTool>, String> {
    manager
        .connect(
            &server_id,
            &name,
            command,
            args,
            env.unwrap_or_default(),
        )
        .await
}

#[tauri::command]
pub async fn mcp_disconnect(
    server_id: String,
    manager: State<'_, Arc<McpManager>>,
) -> Result<(), String> {
    manager.disconnect(&server_id).await
}

#[tauri::command]
pub async fn mcp_list_connections(
    manager: State<'_, Arc<McpManager>>,
) -> Result<Vec<McpConnectionInfo>, String> {
    Ok(manager.list_connections().await)
}

#[tauri::command]
pub async fn mcp_list_tools(
    manager: State<'_, Arc<McpManager>>,
) -> Result<Vec<McpToolInfo>, String> {
    Ok(manager.list_tools_info().await)
}

#[tauri::command]
pub async fn mcp_call_tool(
    server_id: String,
    tool_name: String,
    arguments: serde_json::Value,
    manager: State<'_, Arc<McpManager>>,
) -> Result<McpToolResult, String> {
    manager
        .call_tool(&server_id, &tool_name, arguments)
        .await
}

// --- Filesystem MCP config/audit commands ---

#[tauri::command]
pub async fn get_fs_mcp_config(pool: State<'_, Pool>) -> Result<String, String> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT config FROM mcp_servers WHERE id = 'builtin-filesystem'"
    )
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.unwrap_or_else(|| "{}".to_string()))
}

#[tauri::command]
pub async fn update_fs_mcp_config(
    pool: State<'_, Pool>,
    manager: State<'_, Arc<McpManager>>,
    fs_server: State<'_, Arc<BuiltinFsServer>>,
    config: String,
) -> Result<(), String> {
    // Validate JSON
    let parsed: FsMcpConfig = serde_json::from_str(&config)
        .map_err(|e| format!("Invalid config JSON: {}", e))?;

    sqlx::query("UPDATE mcp_servers SET config = ? WHERE id = 'builtin-filesystem'")
        .bind(&config)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Update the live server config
    fs_server.update_config(parsed).await;

    // If connected, reconnect to refresh tools
    let connections = manager.list_connections().await;
    let is_connected = connections.iter().any(|c| c.id == "builtin-filesystem");
    if is_connected {
        let _ = manager.disconnect("builtin-filesystem").await;
        let _ = manager
            .connect_builtin("builtin-filesystem", "Filesystem", fs_server.inner().clone())
            .await;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_fs_audit_log(
    pool: State<'_, Pool>,
    limit: i64,
    offset: i64,
) -> Result<Vec<FsAuditEntry>, String> {
    builtin_fs_server::get_audit_log(pool.inner(), limit, offset).await
}

#[tauri::command]
pub async fn clear_fs_audit_log(pool: State<'_, Pool>) -> Result<(), String> {
    builtin_fs_server::clear_audit_log(pool.inner()).await
}

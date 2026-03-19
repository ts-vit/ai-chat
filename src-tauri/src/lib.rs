// точка входа, регистрация плагинов и команд
mod commands;
mod models;
mod services;

use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

pub type AgentCancelTokens = Arc<RwLock<HashMap<String, CancellationToken>>>;
pub type TelegramRunNotifier = Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<String>>>>;

use commands::attachments::save_attachment;
use commands::chat::{send_message, stop_comparison_generation, stop_generation, StreamState};
use commands::comparisons::{
    create_comparison, delete_comparison, get_all_comparisons, get_comparison,
    get_comparison_messages, save_comparison_message, stream_comparison_responses,
    update_comparison_message_content, update_comparison_message_usage, update_comparison_title,
};
use commands::database::{
    create_chat, create_image_style, delete_chat, delete_image_style, delete_messages_after,
    get_all_chats, get_image_styles, get_messages, get_messages_branched, save_message,
    switch_branch, update_chat_image_config, update_chat_model, update_chat_negative_prompt,
    update_chat_params, update_chat_title, update_image_style, update_message_content,
    update_message_usage,
};
use commands::folders::{
    create_folder, delete_folder, get_all_folders, move_chat_to_folder, reorder_folders,
    update_folder,
};
use commands::presets::{
    create_preset, delete_preset, get_all_presets, get_chat_system_prompt, set_chat_system_prompt,
    update_preset,
};
use commands::ollama::{
    check_ollama_status, delete_ollama_model, get_local_ollama_models, pull_ollama_model,
};
use commands::providers::{
    create_custom_provider, delete_custom_provider, get_custom_providers, get_provider_chat_count,
    update_custom_provider,
};
use commands::settings::{
    fetch_custom_provider_models, get_balance, get_credits, get_mode_settings, get_models,
    get_ollama_models, load_settings, save_settings, test_proxy, update_mode_settings,
    validate_openrouter_key,
};
use commands::embeddings::{index_message, reindex_all};
use commands::search::{
    delete_message_index, delete_search_index, get_indexing_status, search_messages,
};
use commands::snippets::{
    create_category, create_snippet, delete_category, delete_snippet, get_all_categories,
    get_all_snippets, get_snippets_by_category, get_welcome_snippets, update_category,
    update_snippet,
};
use commands::mcp::{
    mcp_add_server, mcp_call_tool, mcp_connect, mcp_connect_by_id, mcp_disconnect,
    mcp_get_servers, mcp_list_connections, mcp_list_tools, mcp_remove_server, mcp_toggle_server,
    mcp_update_server, get_fs_mcp_config, update_fs_mcp_config, get_fs_audit_log, clear_fs_audit_log,
};
use commands::templates::{
    create_template, delete_template, get_all_templates, reorder_templates, update_template,
};
use commands::export_import::{
    export_chat_json, export_chat_markdown, export_all_chats, import_chats,
};
use commands::audio::{
    check_vosk_status, download_vosk_library, download_vosk_model, get_available_vosk_models,
    is_recording, start_recording, stop_recording_and_transcribe, tts_get_voices, tts_speak,
    tts_stop, uninstall_vosk,
};
use commands::tokens::count_tokens;
use commands::terminal::{get_current_proxy_url, terminal_create, terminal_kill, terminal_resize, terminal_write};
use commands::web_search::{update_message_web_sources, web_search};
use commands::ssh_tunnel::{
    ssh_remove_known_host, ssh_tunnel_connect, ssh_tunnel_disconnect, ssh_tunnel_status,
};
use commands::model_manager::{
    delete_downloaded_model, download_embedding_model, get_model_status,
};
use commands::prompt_library::{
    create_prompt_library_item, delete_prompt_library_item, get_prompt_library,
    seed_builtin_prompts, update_prompt_library_item,
};
use commands::agent::{
    send_agent_message, cancel_agent_run, resume_agent_run, get_agent_runs,
    get_sub_agent_runs,
};
use commands::agent_memory::{
    list_agent_memories, create_agent_memory, update_agent_memory,
    delete_agent_memory, delete_all_agent_memories, search_agent_memories,
};
use commands::skills::{
    list_skills, create_skill, update_skill, delete_skill,
    get_chat_skills, attach_skill_to_chat, detach_skill_from_chat,
    detect_skill_for_message,
};
use commands::budget::get_budget_status_command;
use commands::model_catalog::{
    get_model_catalog, sync_model_catalog, update_model_catalog_entry,
};
use commands::routing::{
    create_routing_rule, delete_routing_rule, get_routing_rules,
    reset_routing_rules_to_defaults, update_routing_rule,
};
use commands::planner::{
    generate_plan, get_plan, get_plans_for_chat, get_plans_for_project, update_task,
    delete_plan, approve_plan, start_plan_execution, execute_single_task,
};
use commands::workspace::{
    list_workspace_artifacts, get_workspace_artifact, create_workspace_artifact,
    update_workspace_artifact, delete_workspace_artifact, delete_chat_workspace,
};
use commands::projects::{
    create_project, get_project, list_projects, update_project,
    delete_project, archive_project, assign_chat_to_project, remove_chat_from_project,
};
use commands::telegram::{
    start_telegram_bot, stop_telegram_bot, get_telegram_status,
    authorize_telegram_user, revoke_telegram_user, validate_telegram_token,
};
use commands::scheduler::{
    list_scheduled_tasks, create_scheduled_task, update_scheduled_task,
    delete_scheduled_task, toggle_scheduled_task, run_scheduled_task_now,
    get_scheduler_status,
};
use commands::knowledge_base::{
    list_knowledge_bases, get_knowledge_base, create_knowledge_base,
    update_knowledge_base, delete_knowledge_base, list_kb_documents,
    add_kb_document, remove_kb_document, add_kb_documents_bulk, get_kb_stats,
    index_kb_document, index_all_kb_documents, reindex_knowledge_base,
    search_knowledge_base, attach_kb_to_chat, detach_kb_from_chat, get_chat_kb,
    export_knowledge_base, import_knowledge_base,
};
use services::telegram_bot::TelegramBotManager;
use services::scheduler::SchedulerManager;
use services::audio_recorder::AudioRecorder;
use services::terminal::TerminalManager;
use services::mcp_manager::McpManager;
use services::ssh_tunnel::SshTunnelManager;
use services::builtin_fs_server::BuiltinFsServer;
use models::mcp::FsMcpConfig;
use tauri_plugin_store::StoreExt;
use sqlx::sqlite::SqlitePool;
use services::embedding_engine::EmbeddingEngine;
use services::model_manager;
use services::vector_store::VectorStore;
use services::memory_vector_store::MemoryVectorStore;
use services::kb_vector_store::KbVectorStore;

pub(crate) const DB_MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS chats (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        chat_id TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        parent_id TEXT,
        timestamp INTEGER NOT NULL,
        FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS presets (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        content TEXT NOT NULL,
        is_default INTEGER DEFAULT 0,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS categories (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS snippets (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        content TEXT NOT NULL,
        category_id TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        FOREIGN KEY (category_id) REFERENCES categories(id)
    )",
    "CREATE TABLE IF NOT EXISTS custom_providers (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        base_url TEXT NOT NULL,
        api_key TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL
    )",
    "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(message_id, content)",
    "CREATE TABLE IF NOT EXISTS folders (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        color TEXT,
        sort_order INTEGER DEFAULT 0,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS chat_templates (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        icon TEXT NOT NULL DEFAULT '💬',
        provider_id TEXT NOT NULL DEFAULT 'openrouter',
        model TEXT NOT NULL DEFAULT '',
        system_prompt TEXT NOT NULL DEFAULT '',
        temperature REAL,
        max_tokens INTEGER,
        top_p REAL,
        top_k INTEGER,
        frequency_penalty REAL,
        presence_penalty REAL,
        sort_order INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS mcp_servers (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        command TEXT NOT NULL,
        args TEXT NOT NULL DEFAULT '[]',
        env TEXT NOT NULL DEFAULT '{}',
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS image_styles (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        prompt_suffix TEXT NOT NULL,
        is_builtin INTEGER NOT NULL DEFAULT 0,
        sort_order INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS comparisons (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL DEFAULT 'New comparison',
        left_provider_id TEXT NOT NULL,
        left_model TEXT NOT NULL,
        left_system_prompt TEXT,
        right_provider_id TEXT NOT NULL,
        right_model TEXT NOT NULL,
        right_system_prompt TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS comparison_messages (
        id TEXT PRIMARY KEY,
        comparison_id TEXT NOT NULL REFERENCES comparisons(id) ON DELETE CASCADE,
        role TEXT NOT NULL,
        side TEXT,
        content TEXT NOT NULL,
        timestamp INTEGER NOT NULL,
        model TEXT,
        prompt_tokens INTEGER DEFAULT 0,
        completion_tokens INTEGER DEFAULT 0,
        cost REAL DEFAULT 0.0
    )",
    "CREATE TABLE IF NOT EXISTS prompt_library (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        content TEXT NOT NULL,
        category TEXT NOT NULL DEFAULT 'general',
        is_builtin INTEGER NOT NULL DEFAULT 0,
        language TEXT NOT NULL DEFAULT 'en',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS mode_settings (
        mode TEXT PRIMARY KEY,
        enabled INTEGER NOT NULL DEFAULT 1,
        sort_order INTEGER NOT NULL DEFAULT 0,
        config TEXT NOT NULL DEFAULT '{}'
    )",
    "CREATE TABLE IF NOT EXISTS agent_runs (
        id TEXT PRIMARY KEY,
        chat_id TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'running',
        iterations INTEGER NOT NULL DEFAULT 0,
        max_iterations INTEGER NOT NULL DEFAULT 25,
        started_at INTEGER NOT NULL,
        finished_at INTEGER,
        error TEXT,
        FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS agent_memory (
        id TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        category TEXT NOT NULL DEFAULT 'fact',
        source_chat_id TEXT,
        source_message_id TEXT,
        is_pinned INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_agent_memory_category ON agent_memory(category)",
    "CREATE INDEX IF NOT EXISTS idx_agent_memory_created ON agent_memory(created_at)",
    "CREATE VIRTUAL TABLE IF NOT EXISTS agent_memory_fts USING fts5(memory_id, content)",
    "CREATE TABLE IF NOT EXISTS skills (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        icon TEXT NOT NULL DEFAULT 'code',
        content TEXT NOT NULL DEFAULT '',
        trigger_description TEXT NOT NULL DEFAULT '',
        required_tools TEXT NOT NULL DEFAULT '[]',
        is_builtin INTEGER NOT NULL DEFAULT 0,
        enabled INTEGER NOT NULL DEFAULT 1,
        sort_order INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS chat_skills (
        chat_id TEXT NOT NULL,
        skill_id TEXT NOT NULL,
        attached_by TEXT NOT NULL DEFAULT 'manual',
        created_at INTEGER NOT NULL,
        PRIMARY KEY (chat_id, skill_id),
        FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE,
        FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS agent_plans (
        id TEXT PRIMARY KEY,
        chat_id TEXT NOT NULL,
        goal TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'draft',
        execution_mode TEXT NOT NULL DEFAULT 'manual',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS agent_tasks (
        id TEXT PRIMARY KEY,
        plan_id TEXT NOT NULL,
        title TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL DEFAULT 'pending',
        dependencies TEXT NOT NULL DEFAULT '[]',
        result TEXT,
        agent_run_id TEXT,
        sort_order INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (plan_id) REFERENCES agent_plans(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS model_catalog (
        id TEXT PRIMARY KEY,
        provider TEXT NOT NULL,
        model_id TEXT NOT NULL,
        display_name TEXT NOT NULL,
        cost_per_input_token REAL,
        cost_per_output_token REAL,
        context_window INTEGER,
        category TEXT NOT NULL DEFAULT 'general',
        strengths TEXT,
        is_available INTEGER NOT NULL DEFAULT 1,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS cost_ledger (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp INTEGER NOT NULL,
        chat_id TEXT NOT NULL,
        agent_run_id TEXT,
        plan_id TEXT,
        model_id TEXT NOT NULL,
        provider TEXT NOT NULL,
        prompt_tokens INTEGER NOT NULL DEFAULT 0,
        completion_tokens INTEGER NOT NULL DEFAULT 0,
        cost REAL NOT NULL DEFAULT 0.0,
        source TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS routing_rules (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task_category TEXT NOT NULL,
        preferred_model TEXT NOT NULL,
        fallback_model TEXT,
        priority INTEGER NOT NULL DEFAULT 0,
        enabled INTEGER NOT NULL DEFAULT 1
    )",
    "CREATE TABLE IF NOT EXISTS workspace_artifacts (
        id TEXT PRIMARY KEY,
        chat_id TEXT NOT NULL,
        project_id TEXT,
        name TEXT NOT NULL,
        content_type TEXT NOT NULL DEFAULT 'text',
        content TEXT,
        file_path TEXT,
        created_by TEXT,
        updated_by TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_workspace_artifacts_chat ON workspace_artifacts(chat_id)",
    "CREATE INDEX IF NOT EXISTS idx_workspace_artifacts_project ON workspace_artifacts(project_id)",
    "CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        goal TEXT DEFAULT '',
        status TEXT NOT NULL DEFAULT 'active',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status)",
    "CREATE TABLE IF NOT EXISTS scheduled_tasks (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        prompt TEXT NOT NULL,
        cron_expression TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        mode TEXT NOT NULL DEFAULT 'assistant',
        model TEXT,
        skill_id TEXT,
        project_id TEXT,
        deliver_telegram INTEGER NOT NULL DEFAULT 1,
        deliver_desktop_notification INTEGER NOT NULL DEFAULT 1,
        last_run_at INTEGER,
        next_run_at INTEGER,
        last_run_status TEXT,
        last_run_error TEXT,
        run_count INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS knowledge_bases (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        embedding_model TEXT NOT NULL DEFAULT 'e5-small',
        chunking_strategy TEXT NOT NULL DEFAULT 'tokens',
        chunk_size INTEGER NOT NULL DEFAULT 512,
        chunk_overlap INTEGER NOT NULL DEFAULT 50,
        retrieval_top_k INTEGER NOT NULL DEFAULT 5,
        retrieval_min_score REAL NOT NULL DEFAULT 0.7,
        system_prompt TEXT NOT NULL DEFAULT '',
        version INTEGER NOT NULL DEFAULT 1,
        status TEXT NOT NULL DEFAULT 'active',
        document_count INTEGER NOT NULL DEFAULT 0,
        total_chunks INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS kb_documents (
        id TEXT PRIMARY KEY,
        kb_id TEXT NOT NULL,
        name TEXT NOT NULL,
        source_type TEXT NOT NULL DEFAULT 'file',
        source_path TEXT,
        source_url TEXT,
        mime_type TEXT NOT NULL DEFAULT 'text/plain',
        file_size INTEGER NOT NULL DEFAULT 0,
        chunk_count INTEGER NOT NULL DEFAULT 0,
        indexing_status TEXT NOT NULL DEFAULT 'pending',
        indexing_error TEXT,
        content_hash TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS kb_chunks (
        id TEXT PRIMARY KEY,
        kb_id TEXT NOT NULL,
        document_id TEXT NOT NULL,
        content TEXT NOT NULL,
        chunk_index INTEGER NOT NULL,
        start_offset INTEGER,
        end_offset INTEGER,
        metadata TEXT NOT NULL DEFAULT '{}',
        created_at INTEGER NOT NULL,
        FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE,
        FOREIGN KEY (document_id) REFERENCES kb_documents(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_kb_documents_kb_id ON kb_documents(kb_id)",
    "CREATE INDEX IF NOT EXISTS idx_kb_chunks_document_id ON kb_chunks(document_id)",
    "CREATE INDEX IF NOT EXISTS idx_kb_chunks_kb_id ON kb_chunks(kb_id)",
];

pub(crate) const ALTER_QUERIES: &[&str] = &[
    "ALTER TABLE messages ADD COLUMN model TEXT DEFAULT ''",
    "ALTER TABLE messages ADD COLUMN prompt_tokens INTEGER DEFAULT 0",
    "ALTER TABLE messages ADD COLUMN completion_tokens INTEGER DEFAULT 0",
    "ALTER TABLE messages ADD COLUMN cost REAL DEFAULT 0.0",
    "ALTER TABLE chats ADD COLUMN system_prompt TEXT DEFAULT ''",
    "ALTER TABLE messages ADD COLUMN has_attachments INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE chats ADD COLUMN provider_id TEXT NOT NULL DEFAULT 'openrouter'",
    "ALTER TABLE chats ADD COLUMN model TEXT DEFAULT ''",
    "ALTER TABLE messages ADD COLUMN fts_indexed INTEGER DEFAULT 0",
    "ALTER TABLE chats ADD COLUMN folder_id TEXT REFERENCES folders(id) ON DELETE SET NULL",
    "ALTER TABLE chats ADD COLUMN is_image_model INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE chats ADD COLUMN temperature REAL",
    "ALTER TABLE chats ADD COLUMN max_tokens INTEGER",
    "ALTER TABLE chats ADD COLUMN top_p REAL",
    "ALTER TABLE chats ADD COLUMN top_k INTEGER",
    "ALTER TABLE chats ADD COLUMN frequency_penalty REAL",
    "ALTER TABLE chats ADD COLUMN presence_penalty REAL",
    "ALTER TABLE messages ADD COLUMN web_sources TEXT DEFAULT NULL",
    "ALTER TABLE snippets ADD COLUMN show_on_welcome INTEGER DEFAULT 0",
    "ALTER TABLE comparison_messages ADD COLUMN has_attachments INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE custom_providers ADD COLUMN api_key TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE chats ADD COLUMN image_size TEXT",
    "ALTER TABLE chats ADD COLUMN image_quality TEXT",
    "ALTER TABLE chats ADD COLUMN image_style TEXT",
    "ALTER TABLE chats ADD COLUMN image_n INTEGER",
    "ALTER TABLE chats ADD COLUMN negative_prompt TEXT",
    "ALTER TABLE chats ADD COLUMN active_child_map TEXT DEFAULT '{}'",
    "ALTER TABLE chats ADD COLUMN branch_migrated INTEGER DEFAULT 0",
    "ALTER TABLE mcp_servers ADD COLUMN server_type TEXT DEFAULT 'stdio'",
    "ALTER TABLE mcp_servers ADD COLUMN config TEXT DEFAULT '{}'",
    "ALTER TABLE chats ADD COLUMN mode TEXT NOT NULL DEFAULT 'chat'",
    "ALTER TABLE folders ADD COLUMN mode TEXT NOT NULL DEFAULT 'chat'",
    "ALTER TABLE chats ADD COLUMN agent_max_iterations INTEGER NOT NULL DEFAULT 25",
    "ALTER TABLE chats ADD COLUMN agent_auto_mode INTEGER NOT NULL DEFAULT 1",
    "ALTER TABLE messages ADD COLUMN agent_step INTEGER",
    "ALTER TABLE messages ADD COLUMN agent_run_id TEXT",
    "ALTER TABLE chats ADD COLUMN auto_skill_detection INTEGER NOT NULL DEFAULT 1",
    "ALTER TABLE agent_plans ADD COLUMN replan_count INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE agent_runs ADD COLUMN prompt_tokens INTEGER DEFAULT 0",
    "ALTER TABLE agent_runs ADD COLUMN completion_tokens INTEGER DEFAULT 0",
    "ALTER TABLE agent_runs ADD COLUMN cost REAL DEFAULT 0.0",
    "ALTER TABLE agent_runs ADD COLUMN assigned_model TEXT",
    "ALTER TABLE agent_tasks ADD COLUMN category TEXT",
    "ALTER TABLE agent_tasks ADD COLUMN assigned_model TEXT",
    "ALTER TABLE agent_runs ADD COLUMN parent_run_id TEXT",
    "ALTER TABLE agent_runs ADD COLUMN orchestrator_chat_id TEXT",
    "ALTER TABLE agent_runs ADD COLUMN spawn_config TEXT",
    "ALTER TABLE agent_runs ADD COLUMN depth INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE chats ADD COLUMN project_id TEXT",
    "ALTER TABLE agent_memory ADD COLUMN project_id TEXT",
    "ALTER TABLE agent_plans ADD COLUMN project_id TEXT",
    "ALTER TABLE chats ADD COLUMN is_telegram_chat INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE chats ADD COLUMN scheduled_task_id TEXT",
    // KB RAG migrations
    "ALTER TABLE chats ADD COLUMN kb_id TEXT",
    "ALTER TABLE messages ADD COLUMN rag_sources TEXT",
    "ALTER TABLE knowledge_bases ADD COLUMN embedding_dimensions INTEGER NOT NULL DEFAULT 384",
    "ALTER TABLE knowledge_bases ADD COLUMN min_chunk_size INTEGER NOT NULL DEFAULT 50",
    "ALTER TABLE knowledge_bases ADD COLUMN query_rewriting_enabled INTEGER NOT NULL DEFAULT 1",
    "ALTER TABLE knowledge_bases ADD COLUMN query_decomposition_enabled INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE knowledge_bases ADD COLUMN query_max_variants INTEGER NOT NULL DEFAULT 3",
    // KB Reranking migrations
    "ALTER TABLE knowledge_bases ADD COLUMN reranker_type TEXT NOT NULL DEFAULT 'none'",
    "ALTER TABLE knowledge_bases ADD COLUMN reranker_overfetch_factor INTEGER NOT NULL DEFAULT 4",
    // KB Context Optimization migrations
    "ALTER TABLE knowledge_bases ADD COLUMN context_token_budget INTEGER NOT NULL DEFAULT 4000",
    "ALTER TABLE knowledge_bases ADD COLUMN context_sentence_extraction INTEGER NOT NULL DEFAULT 1",
    "ALTER TABLE knowledge_bases ADD COLUMN context_redundancy_removal INTEGER NOT NULL DEFAULT 1",
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn"),
    )
    .format_timestamp_millis()
    .init();
    log::info!("AI Chat backend started");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let path = app
                .path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .join("database.db");
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let url = format!("sqlite:{}?mode=rwc", path.display());
            let pool = tauri::async_runtime::block_on(async {
                SqlitePool::connect(&url).await.map_err(|e| e.to_string())
            })?;
            tauri::async_runtime::block_on(sqlx::query("PRAGMA journal_mode=WAL").execute(&pool))
                .map_err(|e| e.to_string())?;
            tauri::async_runtime::block_on(sqlx::query("PRAGMA busy_timeout=5000").execute(&pool))
                .map_err(|e| e.to_string())?;
            for sql in DB_MIGRATIONS {
                tauri::async_runtime::block_on(sqlx::query(*sql).execute(&pool))
                    .map_err(|e| e.to_string())?;
            }
            for sql in ALTER_QUERIES {
                let _ = tauri::async_runtime::block_on(sqlx::query(sql).execute(&pool));
            }

            // Seed default routing rules if empty
            let _ = tauri::async_runtime::block_on(async {
                let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM routing_rules")
                    .fetch_one(&pool)
                    .await
                    .unwrap_or((0,));
                if count.0 == 0 {
                    for (cat, priority) in [
                        ("coding", 10),
                        ("writing", 9),
                        ("analysis", 8),
                        ("vision", 7),
                        ("fast", 6),
                        ("general", 0),
                    ] {
                        let _ = sqlx::query(
                            "INSERT INTO routing_rules (task_category, preferred_model, fallback_model, priority, enabled) VALUES (?, '', NULL, ?, 1)",
                        )
                        .bind(cat)
                        .bind(priority)
                        .execute(&pool)
                        .await;
                    }
                }
                Ok::<(), sqlx::Error>(())
            });

            // Filesystem MCP: audit log table
            let _ = tauri::async_runtime::block_on(
                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS fs_audit_log (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        timestamp INTEGER NOT NULL,
                        tool_name TEXT NOT NULL,
                        path TEXT NOT NULL,
                        result TEXT NOT NULL,
                        details TEXT DEFAULT '',
                        chat_id TEXT DEFAULT ''
                    )"
                ).execute(&pool)
            );

            // Auto-register built-in filesystem server (disabled by default)
            let _ = tauri::async_runtime::block_on(
                sqlx::query(
                    "INSERT OR IGNORE INTO mcp_servers (id, name, server_type, command, args, env, enabled, config, created_at) VALUES ('builtin-filesystem', 'Filesystem', 'builtin', '', '[]', '{}', 0, '{\"allowedDirectories\":[],\"blockedPatterns\":[\".env\",\".ssh\",\"*.key\",\"*.pem\",\"id_rsa\",\".git/config\"],\"readOnly\":false,\"maxFileSizeBytes\":10485760,\"confirmDestructive\":true}', strftime('%s','now'))"
                ).execute(&pool)
            );

            // Backfill parent_id chain for existing messages (one-time migration)
            {
                use sqlx::Row;
                use std::collections::HashMap;
                let chats_to_migrate: Vec<sqlx::sqlite::SqliteRow> = tauri::async_runtime::block_on(
                    sqlx::query("SELECT id FROM chats WHERE branch_migrated = 0")
                        .fetch_all(&pool)
                ).unwrap_or_default();

                if !chats_to_migrate.is_empty() {
                    log::info!("[migration] Backfilling parent_id for {} chats", chats_to_migrate.len());
                }

                let mut migrated_count = 0u32;
                let mut total_messages = 0u32;

                for chat_row in &chats_to_migrate {
                    let chat_id: String = chat_row.get("id");
                    let msg_rows: Vec<sqlx::sqlite::SqliteRow> = tauri::async_runtime::block_on(
                        sqlx::query("SELECT id FROM messages WHERE chat_id = ? ORDER BY timestamp ASC")
                            .bind(&chat_id)
                            .fetch_all(&pool)
                    ).unwrap_or_default();

                    let msg_count = msg_rows.len();
                    let mut active_child_map: HashMap<String, String> = HashMap::new();
                    let mut prev_id: Option<String> = None;
                    let mut had_error = false;

                    for msg_row in &msg_rows {
                        let msg_id: String = msg_row.get("id");
                        if let Err(e) = tauri::async_runtime::block_on(
                            sqlx::query("UPDATE messages SET parent_id = ? WHERE id = ?")
                                .bind(&prev_id)
                                .bind(&msg_id)
                                .execute(&pool)
                        ) {
                            log::error!("[migration] Failed to set parent_id for message {}: {}", msg_id, e);
                            had_error = true;
                        }
                        if let Some(ref pid) = prev_id {
                            active_child_map.insert(pid.clone(), msg_id.clone());
                        }
                        prev_id = Some(msg_id);
                    }

                    let map_json = serde_json::to_string(&active_child_map).unwrap_or_else(|_| "{}".to_string());
                    if let Err(e) = tauri::async_runtime::block_on(
                        sqlx::query("UPDATE chats SET active_child_map = ?, branch_migrated = 1 WHERE id = ?")
                            .bind(&map_json)
                            .bind(&chat_id)
                            .execute(&pool)
                    ) {
                        log::error!("[migration] Failed to update chat {} active_child_map: {}", chat_id, e);
                    } else {
                        migrated_count += 1;
                        total_messages += msg_count as u32;
                    }

                    if had_error {
                        log::warn!("[migration] Chat {} had errors during parent_id backfill ({} messages)", chat_id, msg_count);
                    }
                }

                if !chats_to_migrate.is_empty() {
                    log::info!("[migration] Backfill complete: {}/{} chats migrated, {} messages processed",
                        migrated_count, chats_to_migrate.len(), total_messages);
                }
            }

            // Seed built-in image styles
            let image_style_seeds = [
                ("photo", "Photo", "photorealistic, professional photography, 8k, highly detailed", 1),
                ("pencil", "Pencil Sketch", "pencil sketch, hand-drawn, graphite drawing, detailed shading", 2),
                ("watercolor", "Watercolor", "watercolor painting, soft colors, artistic, flowing", 3),
                ("oil", "Oil Painting", "oil painting, canvas texture, rich colors, classical art", 4),
                ("anime", "Anime", "anime style, cel shading, vibrant colors, manga art", 5),
                ("pixel", "Pixel Art", "pixel art, 16-bit style, retro gaming", 6),
                ("3d", "3D Render", "3D render, CGI, octane render, volumetric lighting", 7),
                ("comic", "Comic", "comic book style, bold lines, cel shading, pop art", 8),
                ("minimal", "Minimalist", "minimalist, clean, simple shapes, flat design", 9),
            ];
            for (id, name, suffix, order) in image_style_seeds {
                let _ = tauri::async_runtime::block_on(
                    sqlx::query("INSERT OR IGNORE INTO image_styles (id, name, prompt_suffix, is_builtin, sort_order, created_at) VALUES (?, ?, ?, 1, ?, strftime('%s','now'))")
                        .bind(id).bind(name).bind(suffix).bind(order)
                        .execute(&pool)
                );
            }

            // Seed built-in prompts
            {
                for (id, title, desc, content, category, lang) in commands::prompt_library::BUILTIN_PROMPTS_DATA {
                    let _ = tauri::async_runtime::block_on(
                        sqlx::query("INSERT OR IGNORE INTO prompt_library (id, title, description, content, category, is_builtin, language, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, ?, strftime('%s','now'), strftime('%s','now'))")
                            .bind(id).bind(title).bind(desc).bind(content).bind(category).bind(lang)
                            .execute(&pool)
                    );
                }
            }

            // Seed default mode settings
            let _ = tauri::async_runtime::block_on(
                sqlx::query(
                    "INSERT OR IGNORE INTO mode_settings (mode, enabled, sort_order, config) VALUES ('chat', 1, 0, '{}'), ('assistant', 1, 1, '{}')"
                ).execute(&pool)
            );

            // Seed built-in skills
            {
                for (i, (id, name, desc, icon, content, trigger_desc, req_tools)) in commands::skills::BUILTIN_SKILLS_DATA.iter().enumerate() {
                    let _ = tauri::async_runtime::block_on(
                        sqlx::query("INSERT OR IGNORE INTO skills (id, name, description, icon, content, trigger_description, required_tools, is_builtin, enabled, sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 1, 1, ?, strftime('%s','now'), strftime('%s','now'))")
                            .bind(id).bind(name).bind(desc).bind(icon).bind(content).bind(trigger_desc).bind(req_tools).bind(i as i64)
                            .execute(&pool)
                    );
                }
            }

            // FTS5 migration: перейти с content-sync (rowid) на standalone (message_id, content).
            let fts_schema: Option<String> = tauri::async_runtime::block_on(
                sqlx::query_scalar(
                    "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'messages_fts'",
                )
                .fetch_optional(&pool),
            )
            .map_err(|e| e.to_string())?;
            let needs_fts_recreate = match fts_schema {
                None => true,
                Some(sql) => {
                    sql.contains("content='messages'")
                        || sql.contains("content_rowid")
                        || !sql.contains("message_id")
                }
            };
            if needs_fts_recreate {
                tauri::async_runtime::block_on(
                    sqlx::query("DROP TABLE IF EXISTS messages_fts").execute(&pool),
                )
                .map_err(|e| e.to_string())?;
                tauri::async_runtime::block_on(
                    sqlx::query(
                        "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(message_id, content)",
                    )
                    .execute(&pool),
                )
                .map_err(|e| e.to_string())?;
                let _ = tauri::async_runtime::block_on(
                    sqlx::query("UPDATE messages SET fts_indexed = 0").execute(&pool),
                );
            }
            app.manage(pool.clone());
            app.manage(StreamState {
                chat_cancel_token: Arc::new(Mutex::new(None)),
                comparison_cancel_token: Arc::new(Mutex::new(None)),
            });
            app.manage(AgentCancelTokens::default());

            // --- ORT Runtime init (load-dynamic) ---
            {
                let resource_path = app.path().resource_dir()
                    .map_err(|e| e.to_string())?;

                #[cfg(target_os = "windows")]
                let dylib_name = "onnxruntime.dll";
                #[cfg(target_os = "linux")]
                let dylib_name = "libonnxruntime.so";
                #[cfg(target_os = "macos")]
                let dylib_name = "libonnxruntime.dylib";

                let ort_path = resource_path.join("ort").join(dylib_name);

                if ort_path.exists() {
                    match ort::init_from(&ort_path) {
                        Ok(builder) => {
                            builder.commit();
                            log::info!("ORT loaded from: {}", ort_path.display());
                        }
                        Err(e) => {
                            log::error!("Failed to init ORT from {}: {}", ort_path.display(), e);
                        }
                    }
                } else {
                    let dev_path = std::env::current_dir()
                        .ok()
                        .map(|d| d.join("resources").join("ort").join(dylib_name))
                        .filter(|p| p.exists());
                    if let Some(dev_ort) = dev_path {
                        match ort::init_from(&dev_ort) {
                            Ok(builder) => {
                                builder.commit();
                                log::info!("ORT loaded from dev path: {}", dev_ort.display());
                            }
                            Err(e) => {
                                log::error!("Failed to init ORT from {}: {}", dev_ort.display(), e);
                            }
                        }
                    } else {
                        log::warn!("ORT dylib not found, semantic search will be disabled");
                    }
                }
            }

            // Embedding Engine
            {
                let resource_dir = app.path().resource_dir()
                    .map_err(|e| e.to_string())?;
                let app_data_dir = app.path().app_data_dir()
                    .map_err(|e| e.to_string())?;

                let app_model_dir = app_data_dir.join("models").join("multilingual-e5-small");
                let bundled_model_dir = resource_dir.join("models").join("multilingual-e5-small");

                // Dev fallback: src-tauri/resources/models/... via current_dir
                let dev_model_dir = std::env::current_dir()
                    .ok()
                    .map(|d| d.join("resources").join("models").join("multilingual-e5-small"))
                    .filter(|p| p.join("model_int8.onnx").exists());

                match EmbeddingEngine::init(app_model_dir, bundled_model_dir) {
                    Ok(_) => log::info!("[Embedding] Engine loaded"),
                    Err(_) => {
                        if let Some(dev_dir) = dev_model_dir {
                            match EmbeddingEngine::init_single(dev_dir.clone()) {
                                Ok(_) => log::info!("[Embedding] Engine loaded from dev path: {}", dev_dir.display()),
                                Err(e) => log::warn!("[Embedding] All paths failed: {} — FTS5 fallback active", e),
                            }
                        } else {
                            log::warn!("[Embedding] Model not found in any location — FTS5 fallback active");
                        }
                    }
                }
            }

            // Model update check (background, non-blocking)
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

                    match model_manager::fetch_manifest(&app_handle).await {
                        Ok(manifest) => {
                            let app_data_dir = match app_handle.path().app_data_dir() {
                                Ok(d) => d,
                                Err(_) => return,
                            };
                            let model_dir = app_data_dir.join("models").join("multilingual-e5-small");
                            let local = model_manager::read_local_version(&model_dir);

                            let update_available = match &local {
                                Some(local_ver) => local_ver.version != manifest.version,
                                None => true,
                            };

                            if update_available {
                                log::info!(
                                    "[ModelManager] Update available: {} -> {}",
                                    local.map(|v| v.version).unwrap_or_else(|| "bundled".to_string()),
                                    manifest.version
                                );
                                let _ = app_handle.emit("model-update-available", &manifest.version);
                            }
                        }
                        Err(e) => {
                            log::debug!("[ModelManager] Manifest check failed (offline?): {}", e);
                        }
                    }
                });
            }

            // Vector Store (LanceDB) — Arc для передачи в tokio::spawn
            let store: Option<VectorStore> = match app.path().app_data_dir() {
                Ok(data_dir) => {
                    let db_path = data_dir.join("lancedb");
                    if let Some(parent) = db_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match tauri::async_runtime::block_on(VectorStore::new(db_path.to_string_lossy().as_ref())) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            eprintln!("VectorStore init failed: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    eprintln!("app_data_dir failed: {}", e);
                    None
                }
            };
            let store_arc = Arc::new(store);
            app.manage(store_arc.clone());

            // Memory Vector Store (LanceDB) — same db_path as main VectorStore
            let memory_store: Option<MemoryVectorStore> = match app.path().app_data_dir() {
                Ok(data_dir) => {
                    let db_path = data_dir.join("lancedb");
                    match tauri::async_runtime::block_on(MemoryVectorStore::new(db_path.to_string_lossy().as_ref())) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            eprintln!("MemoryVectorStore init failed: {}", e);
                            None
                        }
                    }
                }
                Err(_) => None,
            };
            app.manage(Arc::new(memory_store));

            // KB Vector Store (LanceDB) — same db_path as main VectorStore
            let kb_store: Option<KbVectorStore> = match app.path().app_data_dir() {
                Ok(data_dir) => {
                    let db_path = data_dir.join("lancedb");
                    match tauri::async_runtime::block_on(KbVectorStore::new(db_path.to_string_lossy().as_ref())) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            eprintln!("KbVectorStore init failed: {}", e);
                            None
                        }
                    }
                }
                Err(_) => None,
            };
            app.manage(Arc::new(kb_store));

            // KB FTS5 table
            let _ = tauri::async_runtime::block_on(
                services::kb_fts::ensure_kb_fts_table(&pool)
            );

            // KB RAG migrations
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE chats ADD COLUMN kb_id TEXT").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE messages ADD COLUMN rag_sources TEXT").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN embedding_dimensions INTEGER NOT NULL DEFAULT 384").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN min_chunk_size INTEGER NOT NULL DEFAULT 50").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN query_rewriting_enabled INTEGER NOT NULL DEFAULT 1").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN query_decomposition_enabled INTEGER NOT NULL DEFAULT 0").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN query_max_variants INTEGER NOT NULL DEFAULT 3").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN context_token_budget INTEGER NOT NULL DEFAULT 4000").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN context_sentence_extraction INTEGER NOT NULL DEFAULT 1").execute(&pool)
            );
            let _ = tauri::async_runtime::block_on(
                sqlx::query("ALTER TABLE knowledge_bases ADD COLUMN context_redundancy_removal INTEGER NOT NULL DEFAULT 1").execute(&pool)
            );

            let mcp_manager = Arc::new(McpManager::new());
            app.manage(mcp_manager.clone());

            // Built-in Filesystem MCP Server
            let fs_config: FsMcpConfig = {
                let config_json: Option<String> = tauri::async_runtime::block_on(
                    sqlx::query_scalar("SELECT config FROM mcp_servers WHERE id = 'builtin-filesystem'")
                        .fetch_optional(&pool)
                ).unwrap_or(None);
                config_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or(FsMcpConfig {
                        allowed_directories: vec![],
                        blocked_patterns: vec![".env".into(), ".ssh".into(), "*.key".into(), "*.pem".into(), "id_rsa".into(), ".git/config".into()],
                        read_only: false,
                        max_file_size_bytes: 10_485_760,
                        confirm_destructive: true,
                    })
            };
            let fs_server = Arc::new(BuiltinFsServer::new(fs_config, pool.clone()));
            app.manage(fs_server.clone());
            app.manage(std::sync::Mutex::new(TerminalManager::new()));
            app.manage(Arc::new(AudioRecorder::new()));
            app.manage(commands::audio::TtsState::new());

            let ssh_tunnel_manager = Arc::new(SshTunnelManager::new());
            app.manage(ssh_tunnel_manager.clone());

            let telegram_bot_manager = Arc::new(TelegramBotManager::new());
            app.manage(telegram_bot_manager.clone());
            app.manage(TelegramRunNotifier::default());

            let scheduler_manager = Arc::new(SchedulerManager::new());
            app.manage(scheduler_manager.clone());

            // Фоновая индексация при старте: пропускать уже проиндексированные сообщения.
            // Запускается через 5 сек после старта, чтобы UI успел загрузиться.
            // index_message_impl имеет retry при "database is locked" (см. embeddings.rs).
            {
                use std::sync::atomic::Ordering as Ord;
                let pool_startup = pool.clone();
                let store_startup = store_arc.clone();
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let ids: Vec<String> = match sqlx::query_scalar::<_, String>(
                        "SELECT id FROM messages WHERE fts_indexed = 0 AND role IN ('user', 'assistant')",
                    )
                    .fetch_all(&pool_startup)
                    .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("[startup-index] fetch ids failed: {}", e);
                            return;
                        }
                    };
                    if ids.is_empty() {
                        return;
                    }
                    commands::embeddings::INDEXING_IN_PROGRESS.store(true, Ord::Relaxed);
                    let total = ids.len();
                    for (i, batch) in ids.chunks(5).enumerate() {
                        for id in batch {
                            match commands::embeddings::index_message_impl(
                                &pool_startup,
                                store_startup.as_ref().as_ref(),
                                id,
                            )
                            .await
                            {
                                Ok(()) => {}
                                Err(e) => {
                                    eprintln!("[startup-index] failed for {}: {}, skipping", id, e);
                                }
                            }
                        }
                        let indexed = ((i + 1) * 5).min(total);
                        let _ = app_handle.emit(
                            "indexing-progress",
                            serde_json::json!({ "indexed": indexed, "total": total }),
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                    }
                    commands::embeddings::INDEXING_IN_PROGRESS.store(false, Ord::Relaxed);
                    let _ = app_handle.emit("indexing-done", ());
                });
            }

            // MCP: auto-connect enabled servers after startup
            {
                let pool_mcp = pool.clone();
                let mcp_mgr = mcp_manager.clone();
                let fs_srv = fs_server.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let rows = match sqlx::query(
                        "SELECT id, name, command, args, env, enabled, created_at, server_type, config FROM mcp_servers WHERE enabled = 1",
                    )
                    .fetch_all(&pool_mcp)
                    .await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[mcp-autoconnect] fetch servers failed: {}", e);
                            return;
                        }
                    };
                    if rows.is_empty() {
                        return;
                    }
                    log::info!("[mcp-autoconnect] connecting {} server(s)", rows.len());
                    for row in &rows {
                        use sqlx::Row;
                        let id: String = row.get("id");
                        let name: String = row.get("name");
                        let server_type: String = row.try_get("server_type").unwrap_or_else(|_| "stdio".to_string());

                        if server_type == "builtin" {
                            match mcp_mgr.connect_builtin(&id, &name, fs_srv.clone()).await {
                                Ok(tools) => {
                                    log::info!(
                                        "[mcp-autoconnect] connected builtin '{}' ({}) — {} tool(s)",
                                        name, id, tools.len()
                                    );
                                }
                                Err(e) => {
                                    log::error!(
                                        "[mcp-autoconnect] failed to connect builtin '{}' ({}): {}",
                                        name, id, e
                                    );
                                }
                            }
                        } else {
                            let command: String = row.get("command");
                            let args_str: String = row.get("args");
                            let env_str: String = row.get("env");
                            let args: Vec<String> = serde_json::from_str(&args_str).unwrap_or_default();
                            let env: std::collections::HashMap<String, String> =
                                serde_json::from_str(&env_str).unwrap_or_default();
                            match mcp_mgr.connect(&id, &name, command, args, env).await {
                                Ok(tools) => {
                                    log::info!(
                                        "[mcp-autoconnect] connected '{}' ({}) — {} tool(s)",
                                        name, id, tools.len()
                                    );
                                }
                                Err(e) => {
                                    log::error!(
                                        "[mcp-autoconnect] failed to connect '{}' ({}): {}",
                                        name, id, e
                                    );
                                }
                            }
                        }
                    }
                });
            }

            // SSH tunnel auto-connect
            {
                let ssh_mgr = ssh_tunnel_manager.clone();
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let store = match app_handle.store("settings.json") {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    let auto_connect = store.get("sshAutoConnect").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !auto_connect { return; }
                    let host = store.get("sshHost").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let port = store.get("sshPort").and_then(|v| v.as_u64()).map(|v| v as u16).unwrap_or(22);
                    let username = store.get("sshUsername").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let auth_type = store.get("sshAuthType").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "password".into());
                    let password = store.get("sshPassword").and_then(|v| v.as_str().map(String::from));
                    let key_path = store.get("sshKeyPath").and_then(|v| v.as_str().map(String::from));
                    if host.is_empty() || username.is_empty() { return; }
                    let private_key = key_path.and_then(|p| std::fs::read_to_string(&p).ok());
                    match ssh_mgr.connect(app_handle.clone(), host.clone(), port, username, auth_type, password, private_key).await {
                        Ok(local_port) => log::info!("[ssh-autoconnect] connected to {}:{}, local SOCKS5 on 127.0.0.1:{}", host, port, local_port),
                        Err(e) => log::error!("[ssh-autoconnect] failed: {}", e),
                    }
                });
            }

            // Telegram bot auto-start
            {
                let tg_mgr = telegram_bot_manager.clone();
                let app_handle = app.handle().clone();
                let pool_tg = pool.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    let store = match tauri_plugin_store::StoreExt::store(&app_handle, "settings.json") {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    let enabled = store.get("telegramEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
                    let auto_start = store.get("telegramAutoStart").and_then(|v| v.as_bool()).unwrap_or(false);
                    let token = store.get("telegramBotToken").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    if !enabled || !auto_start || token.is_empty() { return; }
                    match tg_mgr.start(app_handle, pool_tg, token).await {
                        Ok(username) => log::info!("[telegram-autostart] bot started: @{}", username),
                        Err(e) => log::error!("[telegram-autostart] failed: {}", e),
                    }
                });
            }

            // Scheduler auto-start (always on when app is running)
            {
                let sched_mgr = scheduler_manager.clone();
                let app_handle = app.handle().clone();
                let pool_sched = pool.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    sched_mgr.start(app_handle, pool_sched).await;
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_attachment,
            send_message,
            stop_generation,
            stop_comparison_generation,
            save_settings,
            load_settings,
            get_models,
            get_ollama_models,
            fetch_custom_provider_models,
            get_balance,
            check_ollama_status,
            get_local_ollama_models,
            pull_ollama_model,
            delete_ollama_model,
            get_credits,
            get_custom_providers,
            create_custom_provider,
            update_custom_provider,
            delete_custom_provider,
            create_chat,
            delete_chat,
            get_all_chats,
            get_all_folders,
            create_folder,
            update_folder,
            delete_folder,
            reorder_folders,
            move_chat_to_folder,
            save_message,
            update_message_content,
            update_message_usage,
            get_messages,
            get_messages_branched,
            switch_branch,
            update_chat_title,
            update_chat_model,
            update_chat_params,
            update_chat_image_config,
            update_chat_negative_prompt,
            get_image_styles,
            create_image_style,
            update_image_style,
            delete_image_style,
            delete_messages_after,
            create_preset,
            update_preset,
            delete_preset,
            get_all_presets,
            set_chat_system_prompt,
            get_chat_system_prompt,
            create_category,
            update_category,
            delete_category,
            get_all_categories,
            create_snippet,
            update_snippet,
            delete_snippet,
            get_all_snippets,
            get_snippets_by_category,
            get_welcome_snippets,
            search_messages,
            get_indexing_status,
            index_message,
            reindex_all,
            delete_search_index,
            delete_message_index,
            export_chat_json,
            export_chat_markdown,
            export_all_chats,
            get_all_templates,
            create_template,
            update_template,
            delete_template,
            reorder_templates,
            import_chats,
            mcp_connect,
            mcp_disconnect,
            mcp_list_connections,
            mcp_list_tools,
            mcp_call_tool,
            mcp_get_servers,
            mcp_add_server,
            mcp_update_server,
            mcp_remove_server,
            mcp_toggle_server,
            mcp_connect_by_id,
            count_tokens,
            start_recording,
            stop_recording_and_transcribe,
            get_available_vosk_models,
            is_recording,
            check_vosk_status,
            download_vosk_library,
            download_vosk_model,
            uninstall_vosk,
            tts_speak,
            tts_stop,
            tts_get_voices,
            create_comparison,
            get_all_comparisons,
            get_comparison,
            delete_comparison,
            update_comparison_title,
            get_comparison_messages,
            save_comparison_message,
            update_comparison_message_usage,
            update_comparison_message_content,
            stream_comparison_responses,
            web_search,
            update_message_web_sources,
            test_proxy,
            validate_openrouter_key,
            get_provider_chat_count,
            terminal_create,
            terminal_write,
            terminal_resize,
            terminal_kill,
            get_current_proxy_url,
            ssh_tunnel_connect,
            ssh_tunnel_disconnect,
            ssh_tunnel_status,
            ssh_remove_known_host,
            get_model_status,
            download_embedding_model,
            delete_downloaded_model,
            commands::injections::read_file_contents,
            get_prompt_library,
            create_prompt_library_item,
            update_prompt_library_item,
            delete_prompt_library_item,
            seed_builtin_prompts,
            get_fs_mcp_config,
            update_fs_mcp_config,
            get_fs_audit_log,
            clear_fs_audit_log,
            get_mode_settings,
            update_mode_settings,
            send_agent_message,
            cancel_agent_run,
            resume_agent_run,
            get_agent_runs,
            get_sub_agent_runs,
            list_agent_memories,
            create_agent_memory,
            update_agent_memory,
            delete_agent_memory,
            delete_all_agent_memories,
            search_agent_memories,
            list_skills,
            create_skill,
            update_skill,
            delete_skill,
            get_chat_skills,
            attach_skill_to_chat,
            detach_skill_from_chat,
            detect_skill_for_message,
            get_budget_status_command,
            sync_model_catalog,
            get_model_catalog,
            update_model_catalog_entry,
            get_routing_rules,
            create_routing_rule,
            update_routing_rule,
            delete_routing_rule,
            reset_routing_rules_to_defaults,
            generate_plan,
            get_plan,
            get_plans_for_chat,
            get_plans_for_project,
            update_task,
            delete_plan,
            approve_plan,
            start_plan_execution,
            execute_single_task,
            list_workspace_artifacts,
            get_workspace_artifact,
            create_workspace_artifact,
            update_workspace_artifact,
            delete_workspace_artifact,
            delete_chat_workspace,
            create_project,
            get_project,
            list_projects,
            update_project,
            delete_project,
            archive_project,
            assign_chat_to_project,
            remove_chat_from_project,
            start_telegram_bot,
            stop_telegram_bot,
            get_telegram_status,
            authorize_telegram_user,
            revoke_telegram_user,
            validate_telegram_token,
            list_scheduled_tasks,
            create_scheduled_task,
            update_scheduled_task,
            delete_scheduled_task,
            toggle_scheduled_task,
            run_scheduled_task_now,
            get_scheduler_status,
            list_knowledge_bases,
            get_knowledge_base,
            create_knowledge_base,
            update_knowledge_base,
            delete_knowledge_base,
            list_kb_documents,
            add_kb_document,
            remove_kb_document,
            add_kb_documents_bulk,
            get_kb_stats,
            index_kb_document,
            index_all_kb_documents,
            reindex_knowledge_base,
            search_knowledge_base,
            attach_kb_to_chat,
            detach_kb_from_chat,
            get_chat_kb,
            export_knowledge_base,
            import_knowledge_base,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod test_db;
#[cfg(test)]
mod tests;
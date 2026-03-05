// точка входа, регистрация плагинов и команд
mod commands;
mod models;
mod services;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use commands::attachments::save_attachment;
use commands::chat::{send_message, stop_generation, StreamState};
use commands::comparisons::{
    create_comparison, delete_comparison, get_all_comparisons, get_comparison,
    get_comparison_messages, save_comparison_message, stream_comparison_responses,
    update_comparison_message_content, update_comparison_message_usage, update_comparison_title,
};
use commands::database::{
    create_chat, delete_chat, delete_messages_after, get_all_chats, get_messages, save_message,
    update_chat_model, update_chat_params, update_chat_title, update_message_content,
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
    create_custom_provider, delete_custom_provider, get_custom_providers, update_custom_provider,
};
use commands::settings::{
    fetch_custom_provider_models, get_balance, get_credits, get_models, get_ollama_models,
    load_settings, save_settings,
};
use commands::embeddings::{index_message, reindex_all};
use commands::search::{
    delete_message_index, delete_search_index, get_indexing_status, search_messages,
};
use commands::snippets::{
    create_category, create_snippet, delete_category, delete_snippet, get_all_categories,
    get_all_snippets, get_snippets_by_category, update_category, update_snippet,
};
use commands::mcp::{
    mcp_add_server, mcp_call_tool, mcp_connect, mcp_connect_by_id, mcp_disconnect,
    mcp_get_servers, mcp_list_connections, mcp_list_tools, mcp_remove_server, mcp_toggle_server,
    mcp_update_server,
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
use services::audio_recorder::AudioRecorder;
use services::mcp_manager::McpManager;
use sqlx::sqlite::SqlitePool;
use services::embedding_engine::EmbeddingEngine;
use services::vector_store::VectorStore;

const DB_MIGRATIONS: &[&str] = &[
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
            let alter_queries = [
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
            ];
            for sql in alter_queries {
                let _ = tauri::async_runtime::block_on(sqlx::query(sql).execute(&pool));
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
                cancel_token: Arc::new(Mutex::new(None)),
            });

            // Embedding Engine (model для семантического поиска)
            let model_dir = {
                // 1. Сначала проверяем resource_dir (production build)
                let from_resources = app.path().resource_dir().ok().map(|d| {
                    d.join("models").join("multilingual-e5-small")
                }).filter(|p| p.exists());

                // 2. Fallback: src-tauri/resources/ (dev mode)
                let from_dev = std::env::current_dir().ok().map(|d| {
                    d.join("resources").join("models").join("multilingual-e5-small")
                }).filter(|p| p.exists());

                from_resources.or(from_dev)
            };
            if let Some(dir) = model_dir {
                match EmbeddingEngine::init(dir.clone()) {
                    Ok(_) => println!("[Embedding] Engine loaded from: {:?}", dir),
                    Err(e) => eprintln!("[Embedding] Init failed: {}", e),
                }
            } else {
                eprintln!("[Embedding] Model dir not found, semantic search disabled");
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
            let mcp_manager = Arc::new(McpManager::new());
            app.manage(mcp_manager.clone());
            app.manage(Arc::new(AudioRecorder::new()));
            app.manage(commands::audio::TtsState::new());

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
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let rows = match sqlx::query(
                        "SELECT id, name, command, args, env, enabled, created_at FROM mcp_servers WHERE enabled = 1",
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
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_attachment,
            send_message,
            stop_generation,
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
            update_chat_title,
            update_chat_model,
            update_chat_params,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
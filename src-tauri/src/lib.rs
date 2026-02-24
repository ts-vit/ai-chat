// точка входа, регистрация плагинов и команд
mod commands;
mod models;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

use commands::chat::{send_message, stop_generation, StreamState};
use commands::database::{
    create_chat, delete_chat, delete_messages_after, get_all_chats, get_messages, save_message,
    update_chat_title, update_message_content, update_message_usage,
};
use commands::presets::{
    create_preset, delete_preset, get_all_presets, get_chat_system_prompt, set_chat_system_prompt,
    update_preset,
};
use commands::settings::{get_balance, get_credits, get_models, load_settings, save_settings};
use commands::snippets::{
    create_category, create_snippet, delete_category, delete_snippet, get_all_categories,
    get_all_snippets, get_snippets_by_category, update_category, update_snippet,
};
use sqlx::sqlite::SqlitePool;

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
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
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
            ];
            for sql in alter_queries {
                let _ = tauri::async_runtime::block_on(sqlx::query(sql).execute(&pool));
            }
            app.manage(pool);
            app.manage(StreamState {
                cancel_token: Arc::new(Mutex::new(None)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_message,
            stop_generation,
            save_settings,
            load_settings,
            get_models,
            get_balance,
            get_credits,
            create_chat,
            delete_chat,
            get_all_chats,
            save_message,
            update_message_content,
            update_message_usage,
            get_messages,
            update_chat_title,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
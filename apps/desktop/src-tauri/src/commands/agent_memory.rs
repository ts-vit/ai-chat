use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::models::memory::{AgentMemory, AgentMemorySearchResult};
use crate::services::embedding_helper::get_embedding_provider;
use crate::services::memory_fts;
use crate::services::memory_vector_store::MemoryVectorStore;
use uni_embedding::EmbeddingProvider;

const DEDUP_THRESHOLD: f32 = 0.85;

/// Shared logic for creating a memory (used by both command and agent tool)
pub async fn create_memory_impl(
    pool: &SqlitePool,
    memory_store: Option<&MemoryVectorStore>,
    content: &str,
    category: &str,
    source_chat_id: Option<&str>,
    source_message_id: Option<&str>,
    project_id: Option<&str>,
    embedding_provider: Option<&dyn EmbeddingProvider>,
) -> Result<AgentMemory, String> {
    let now = uni_common::now_unix_secs();

    // Generate embedding for dedup check
    let embedding = if let Some(p) = embedding_provider {
        let vecs = p
            .embed_documents(&[content.to_string()])
            .await
            .map_err(|e| e.to_string())?;
        vecs.into_iter().next()
    } else {
        None
    };

    // Dedup check: search for similar memory (global — dedup across all projects)
    if let (Some(store), Some(ref emb)) = (memory_store, &embedding) {
        let results = store.search(emb, 1).await.map_err(|e| e.to_string())?;
        if let Some((existing_id, similarity)) = results.first() {
            if *similarity > DEDUP_THRESHOLD {
                // Update existing record
                sqlx::query(
                    "UPDATE agent_memory SET content = ?, category = ?, updated_at = ? WHERE id = ?",
                )
                .bind(content)
                .bind(category)
                .bind(now)
                .bind(existing_id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;

                // Update vector
                store.update(existing_id, emb).await.map_err(|e| e.to_string())?;

                // Update FTS
                memory_fts::memory_fts_index(pool, existing_id, content).await?;

                // Return updated record
                let row = sqlx::query(
                    "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at FROM agent_memory WHERE id = ?"
                )
                .bind(existing_id)
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())?;

                return Ok(AgentMemory {
                    id: row.get("id"),
                    content: row.get("content"),
                    category: row.get("category"),
                    source_chat_id: row.get("source_chat_id"),
                    source_message_id: row.get("source_message_id"),
                    project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
                    is_pinned: row.get::<bool, _>("is_pinned"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                });
            }
        }
    }

    // Insert new record
    let id = uni_common::generate_id();
    sqlx::query(
        "INSERT INTO agent_memory (id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(content)
    .bind(category)
    .bind(source_chat_id)
    .bind(source_message_id)
    .bind(project_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Save embedding
    if let (Some(store), Some(emb)) = (memory_store, &embedding) {
        let _ = store.save(&id, emb).await;
    }

    // Index in FTS
    memory_fts::memory_fts_index(pool, &id, content).await?;

    Ok(AgentMemory {
        id,
        content: content.to_string(),
        category: category.to_string(),
        source_chat_id: source_chat_id.map(String::from),
        source_message_id: source_message_id.map(String::from),
        project_id: project_id.map(String::from),
        is_pinned: false,
        created_at: now,
        updated_at: now,
    })
}

/// Shared logic for searching memories (used by both command and agent tool)
pub async fn search_memories_impl(
    pool: &SqlitePool,
    memory_store: Option<&MemoryVectorStore>,
    query: &str,
    limit: usize,
    project_id: Option<&str>,
    embedding_provider: Option<&dyn EmbeddingProvider>,
) -> Result<Vec<AgentMemorySearchResult>, String> {
    crate::services::memory_search::search_memories(
        pool,
        memory_store,
        query,
        limit,
        project_id,
        embedding_provider,
    )
    .await
}

// ─── Tauri Commands ─────────────────────────────────────────────

#[tauri::command]
pub async fn list_agent_memories(
    pool: State<'_, SqlitePool>,
    category: Option<String>,
    search_text: Option<String>,
    project_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<AgentMemory>, String> {
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    let project_filter = if project_id.is_some() { " AND project_id = ?" } else { "" };

    let rows = if let Some(ref cat) = category {
        let q_str = format!(
            "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at
             FROM agent_memory WHERE category = ?{} ORDER BY is_pinned DESC, updated_at DESC LIMIT ? OFFSET ?",
            project_filter
        );
        let mut q = sqlx::query(&q_str).bind(cat);
        if let Some(ref pid) = project_id { q = q.bind(pid); }
        q.bind(limit).bind(offset).fetch_all(pool.inner()).await.map_err(|e| e.to_string())?
    } else if let Some(ref text) = search_text {
        let fts_results = memory_fts::memory_fts_search(pool.inner(), text, limit).await?;
        let ids: Vec<String> = fts_results.into_iter().map(|(id, _)| id).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let q_str = format!(
            "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at
             FROM agent_memory WHERE id IN ({}){} ORDER BY is_pinned DESC, updated_at DESC",
            placeholders.join(","), project_filter
        );
        let mut q = sqlx::query(&q_str);
        for id in &ids { q = q.bind(id); }
        if let Some(ref pid) = project_id { q = q.bind(pid); }
        q.fetch_all(pool.inner()).await.map_err(|e| e.to_string())?
    } else {
        let q_str = if project_id.is_some() {
            "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at
             FROM agent_memory WHERE project_id = ? ORDER BY is_pinned DESC, updated_at DESC LIMIT ? OFFSET ?".to_string()
        } else {
            "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at
             FROM agent_memory ORDER BY is_pinned DESC, updated_at DESC LIMIT ? OFFSET ?".to_string()
        };
        let mut q = sqlx::query(&q_str);
        if let Some(ref pid) = project_id { q = q.bind(pid); }
        q.bind(limit).bind(offset).fetch_all(pool.inner()).await.map_err(|e| e.to_string())?
    };

    let memories: Vec<AgentMemory> = rows
        .iter()
        .map(|row| AgentMemory {
            id: row.get("id"),
            content: row.get("content"),
            category: row.get("category"),
            source_chat_id: row.get("source_chat_id"),
            source_message_id: row.get("source_message_id"),
            project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
            is_pinned: row.get::<bool, _>("is_pinned"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();
    Ok(memories)
}

#[tauri::command]
pub async fn create_agent_memory(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    content: String,
    category: String,
    source_chat_id: Option<String>,
    source_message_id: Option<String>,
    project_id: Option<String>,
) -> Result<AgentMemory, String> {
    let prov = get_embedding_provider(&app).await;
    let pref = prov.as_ref().map(|b| b.as_ref() as &dyn EmbeddingProvider);
    create_memory_impl(
        pool.inner(),
        memory_store.as_ref().as_ref(),
        &content,
        &category,
        source_chat_id.as_deref(),
        source_message_id.as_deref(),
        project_id.as_deref(),
        pref,
    )
    .await
}

#[tauri::command]
pub async fn update_agent_memory(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    id: String,
    content: Option<String>,
    category: Option<String>,
    is_pinned: Option<bool>,
) -> Result<AgentMemory, String> {
    let now = uni_common::now_unix_secs();

    if let Some(ref c) = content {
        sqlx::query("UPDATE agent_memory SET content = ?, updated_at = ? WHERE id = ?")
            .bind(c)
            .bind(now)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

        if let Some(store) = memory_store.as_ref().as_ref() {
            if let Some(prov) = get_embedding_provider(&app).await {
                if let Ok(vecs) = prov.embed_documents(&[c.clone()]).await {
                    if let Some(emb) = vecs.into_iter().next() {
                        let _ = store.update(&id, &emb).await;
                    }
                }
            }
        }

        // Update FTS
        memory_fts::memory_fts_index(pool.inner(), &id, c).await?;
    }
    if let Some(ref cat) = category {
        sqlx::query("UPDATE agent_memory SET category = ?, updated_at = ? WHERE id = ?")
            .bind(cat)
            .bind(now)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(pinned) = is_pinned {
        sqlx::query("UPDATE agent_memory SET is_pinned = ?, updated_at = ? WHERE id = ?")
            .bind(pinned)
            .bind(now)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    let row = sqlx::query(
        "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at FROM agent_memory WHERE id = ?"
    )
    .bind(&id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(AgentMemory {
        id: row.get("id"),
        content: row.get("content"),
        category: row.get("category"),
        source_chat_id: row.get("source_chat_id"),
        source_message_id: row.get("source_message_id"),
        project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
        is_pinned: row.get::<bool, _>("is_pinned"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[tauri::command]
pub async fn delete_agent_memory(
    pool: State<'_, SqlitePool>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_memory WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    if let Some(store) = memory_store.as_ref().as_ref() {
        let _ = store.delete(&id).await;
    }

    memory_fts::memory_fts_delete(pool.inner(), &id).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_all_agent_memories(
    pool: State<'_, SqlitePool>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_memory")
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    if let Some(store) = memory_store.as_ref().as_ref() {
        let _ = store.delete_all().await;
    }

    memory_fts::memory_fts_delete_all(pool.inner()).await?;
    Ok(())
}

#[tauri::command]
pub async fn search_agent_memories(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<AgentMemorySearchResult>, String> {
    let limit = limit.unwrap_or(10) as usize;
    let prov = get_embedding_provider(&app).await;
    let pref = prov.as_ref().map(|b| b.as_ref() as &dyn EmbeddingProvider);
    search_memories_impl(
        pool.inner(),
        memory_store.as_ref().as_ref(),
        &query,
        limit,
        None,
        pref,
    )
    .await
}

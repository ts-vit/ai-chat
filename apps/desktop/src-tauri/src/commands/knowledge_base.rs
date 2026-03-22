use sqlx::Row;
use std::collections::HashMap;
use std::io::{Read as IoRead, Write as IoWrite};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use zip::ZipArchive;
use serde::{Deserialize, Serialize};

use uni_settings::{JsonSettingsStore, SettingsStore};
use crate::models::knowledge_base::{KbDocument, KbStats, KnowledgeBase};
use crate::services::kb_vector_store::KbVectorStore;
use crate::services::kb_indexer;
use crate::services::kb_fts;
use crate::services::kb_search::{self, KbSearchConfig, KbSearchResultItem};
use crate::services::http_client::build_http_client;

type Pool = sqlx::SqlitePool;
type KbStore = Arc<Option<KbVectorStore>>;

fn get_kb_documents_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("kb_documents");
    Ok(dir)
}

fn detect_mime_type(extension: &str) -> &'static str {
    match extension.to_lowercase().as_str() {
        "txt" | "log" | "cfg" | "ini" => "text/plain",
        "md" | "markdown" | "mdx" => "text/markdown",
        "html" | "htm" => "text/html",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "json" => "application/json",
        "csv" => "text/csv",
        "xml" | "xhtml" => "application/xml",
        "rst" => "text/x-rst",
        "yaml" | "yml" | "toml" => "text/plain",
        "rs" => "text/x-rust",
        "py" => "text/x-python",
        "ts" | "tsx" => "text/x-typescript",
        "js" | "jsx" => "application/javascript",
        "go" => "text/x-go",
        "java" => "text/x-java",
        "c" | "h" => "text/x-c",
        "cpp" | "hpp" => "text/x-c++",
        "cs" => "text/x-csharp",
        "rb" => "text/x-ruby",
        "php" => "text/x-php",
        "swift" => "text/x-swift",
        "kt" => "text/x-kotlin",
        "sh" | "bash" | "zsh" => "text/x-shellscript",
        "lua" => "text/x-lua",
        "scala" => "text/x-scala",
        _ => "text/plain",
    }
}

fn map_kb_row(row: sqlx::sqlite::SqliteRow) -> KnowledgeBase {
    KnowledgeBase {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        embedding_model: row.get("embedding_model"),
        embedding_dimensions: row.try_get("embedding_dimensions").unwrap_or(384),
        chunking_strategy: row.get("chunking_strategy"),
        chunk_size: row.get("chunk_size"),
        chunk_overlap: row.get("chunk_overlap"),
        min_chunk_size: row.try_get("min_chunk_size").unwrap_or(50),
        retrieval_top_k: row.get("retrieval_top_k"),
        retrieval_min_score: row.get("retrieval_min_score"),
        query_rewriting_enabled: row.try_get::<bool, _>("query_rewriting_enabled").unwrap_or(true),
        query_decomposition_enabled: row.try_get::<bool, _>("query_decomposition_enabled").unwrap_or(false),
        query_max_variants: row.try_get("query_max_variants").unwrap_or(3),
        reranker_type: row.try_get("reranker_type").unwrap_or("none".to_string()),
        reranker_overfetch_factor: row.try_get("reranker_overfetch_factor").unwrap_or(4),
        context_token_budget: row.try_get("context_token_budget").unwrap_or(4000),
        context_sentence_extraction: row.try_get::<bool, _>("context_sentence_extraction").unwrap_or(true),
        context_redundancy_removal: row.try_get::<bool, _>("context_redundancy_removal").unwrap_or(true),
        system_prompt: row.get("system_prompt"),
        version: row.get("version"),
        status: row.get("status"),
        document_count: row.get("document_count"),
        total_chunks: row.get("total_chunks"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_doc_row(row: sqlx::sqlite::SqliteRow) -> KbDocument {
    KbDocument {
        id: row.get("id"),
        kb_id: row.get("kb_id"),
        name: row.get("name"),
        source_type: row.get("source_type"),
        source_path: row.get("source_path"),
        source_url: row.get("source_url"),
        mime_type: row.get("mime_type"),
        file_size: row.get("file_size"),
        chunk_count: row.get("chunk_count"),
        indexing_status: row.get("indexing_status"),
        indexing_error: row.get("indexing_error"),
        content_hash: row.get("content_hash"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

// --- KB CRUD ---

#[tauri::command]
pub async fn list_knowledge_bases(pool: State<'_, Pool>) -> Result<Vec<KnowledgeBase>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, embedding_model, embedding_dimensions, chunking_strategy, chunk_size, chunk_overlap, \
         min_chunk_size, retrieval_top_k, retrieval_min_score, reranker_type, reranker_overfetch_factor, \
         context_token_budget, context_sentence_extraction, context_redundancy_removal, \
         system_prompt, version, status, document_count, \
         total_chunks, created_at, updated_at FROM knowledge_bases WHERE notebook_id IS NULL ORDER BY created_at DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(map_kb_row).collect())
}

#[tauri::command]
pub async fn get_knowledge_base(pool: State<'_, Pool>, id: String) -> Result<KnowledgeBase, String> {
    let row = sqlx::query(
        "SELECT id, name, description, embedding_model, embedding_dimensions, chunking_strategy, chunk_size, chunk_overlap, \
         min_chunk_size, retrieval_top_k, retrieval_min_score, reranker_type, reranker_overfetch_factor, \
         context_token_budget, context_sentence_extraction, context_redundancy_removal, \
         system_prompt, version, status, document_count, \
         total_chunks, created_at, updated_at FROM knowledge_bases WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(map_kb_row(row))
}

#[tauri::command]
pub async fn create_knowledge_base(
    pool: State<'_, Pool>,
    name: String,
    description: String,
    embedding_model: Option<String>,
) -> Result<KnowledgeBase, String> {
    let id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();
    let emb_model = embedding_model.unwrap_or_else(|| "openai".to_string());
    let emb_dims = uni_embedding::default_dimensions(match emb_model.as_str() {
        "gemini" | "gemini-embedding" => "gemini",
        _ => "openai",
    }) as i64;

    sqlx::query(
        "INSERT INTO knowledge_bases (id, name, description, embedding_model, embedding_dimensions, \
         chunking_strategy, chunk_size, chunk_overlap, min_chunk_size, retrieval_top_k, retrieval_min_score, \
         system_prompt, version, status, document_count, total_chunks, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, 'auto', 512, 50, 50, 5, 0.7, '', 1, 'active', 0, 0, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&description)
    .bind(&emb_model)
    .bind(emb_dims)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(KnowledgeBase {
        id,
        name,
        description,
        embedding_model: emb_model,
        embedding_dimensions: emb_dims,
        chunking_strategy: "auto".to_string(),
        chunk_size: 512,
        chunk_overlap: 50,
        min_chunk_size: 50,
        retrieval_top_k: 5,
        retrieval_min_score: 0.7,
        query_rewriting_enabled: true,
        query_decomposition_enabled: false,
        query_max_variants: 3,
        reranker_type: "none".to_string(),
        reranker_overfetch_factor: 4,
        context_token_budget: 4000,
        context_sentence_extraction: true,
        context_redundancy_removal: true,
        system_prompt: String::new(),
        version: 1,
        status: "active".to_string(),
        document_count: 0,
        total_chunks: 0,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn update_knowledge_base(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    description: String,
    embedding_model: Option<String>,
    embedding_dimensions: Option<i64>,
    chunking_strategy: String,
    chunk_size: i64,
    chunk_overlap: i64,
    min_chunk_size: Option<i64>,
    retrieval_top_k: i64,
    retrieval_min_score: f64,
    query_rewriting_enabled: Option<bool>,
    query_decomposition_enabled: Option<bool>,
    query_max_variants: Option<i64>,
    reranker_type: Option<String>,
    reranker_overfetch_factor: Option<i64>,
    context_token_budget: Option<i64>,
    context_sentence_extraction: Option<bool>,
    context_redundancy_removal: Option<bool>,
    system_prompt: String,
) -> Result<(), String> {
    let now = uni_common::now_unix_secs();
    let min_cs = min_chunk_size.unwrap_or(50);
    let qr_enabled = query_rewriting_enabled.unwrap_or(true);
    let qd_enabled = query_decomposition_enabled.unwrap_or(false);
    let q_max = query_max_variants.unwrap_or(3);
    let r_type = reranker_type.unwrap_or_else(|| "none".to_string());
    let r_overfetch = reranker_overfetch_factor.unwrap_or(4);
    let ctx_budget = context_token_budget.unwrap_or(4000);
    let ctx_sentence = context_sentence_extraction.unwrap_or(true);
    let ctx_redundancy = context_redundancy_removal.unwrap_or(true);

    sqlx::query(
        "UPDATE knowledge_bases SET name = ?, description = ?, embedding_model = COALESCE(?, embedding_model), \
         embedding_dimensions = COALESCE(?, embedding_dimensions), chunking_strategy = ?, \
         chunk_size = ?, chunk_overlap = ?, min_chunk_size = ?, retrieval_top_k = ?, retrieval_min_score = ?, \
         query_rewriting_enabled = ?, query_decomposition_enabled = ?, query_max_variants = ?, \
         reranker_type = ?, reranker_overfetch_factor = ?, \
         context_token_budget = ?, context_sentence_extraction = ?, context_redundancy_removal = ?, \
         system_prompt = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&description)
    .bind(&embedding_model)
    .bind(embedding_dimensions)
    .bind(&chunking_strategy)
    .bind(chunk_size)
    .bind(chunk_overlap)
    .bind(min_cs)
    .bind(retrieval_top_k)
    .bind(retrieval_min_score)
    .bind(qr_enabled)
    .bind(qd_enabled)
    .bind(q_max)
    .bind(&r_type)
    .bind(r_overfetch)
    .bind(ctx_budget)
    .bind(ctx_sentence)
    .bind(ctx_redundancy)
    .bind(&system_prompt)
    .bind(now)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_knowledge_base(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    id: String,
) -> Result<(), String> {
    // Clean up vector and FTS indexes
    if let Some(store) = kb_store.as_ref() {
        let _ = kb_indexer::delete_kb_index(pool.inner(), store, &id).await;
    }

    // Delete files from disk
    let kb_dir = get_kb_documents_dir(&app_handle)?.join(&id);
    if kb_dir.exists() {
        std::fs::remove_dir_all(&kb_dir).map_err(|e| e.to_string())?;
    }

    // Delete KB row (CASCADE deletes documents and chunks)
    sqlx::query("DELETE FROM knowledge_bases WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// --- Document management ---

#[tauri::command]
pub async fn list_kb_documents(
    pool: State<'_, Pool>,
    kb_id: String,
) -> Result<Vec<KbDocument>, String> {
    let rows = sqlx::query(
        "SELECT id, kb_id, name, source_type, source_path, source_url, mime_type, file_size, \
         chunk_count, indexing_status, indexing_error, content_hash, created_at, updated_at \
         FROM kb_documents WHERE kb_id = ? ORDER BY created_at DESC",
    )
    .bind(&kb_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(map_doc_row).collect())
}

#[tauri::command]
pub async fn add_kb_document(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    kb_id: String,
    file_path: String,
) -> Result<KbDocument, String> {
    let source = std::path::Path::new(&file_path);
    if !source.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mime_type = detect_mime_type(extension).to_string();

    let doc_id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();

    // Create target directory and copy file
    let doc_dir = get_kb_documents_dir(&app_handle)?
        .join(&kb_id)
        .join(&doc_id);
    std::fs::create_dir_all(&doc_dir).map_err(|e| e.to_string())?;

    let target = doc_dir.join(&file_name);
    std::fs::copy(source, &target).map_err(|e| e.to_string())?;

    // Get file size
    let metadata = std::fs::metadata(&target).map_err(|e| e.to_string())?;
    let file_size = metadata.len() as i64;

    // Compute SHA256 hash
    use sha2::{Digest, Sha256};
    let file_bytes = std::fs::read(&target).map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha256::digest(&file_bytes));

    // Store relative path
    let relative_path = format!("kb_documents/{}/{}/{}", kb_id, doc_id, file_name);

    // Insert document record
    sqlx::query(
        "INSERT INTO kb_documents (id, kb_id, name, source_type, source_path, mime_type, \
         file_size, chunk_count, indexing_status, content_hash, created_at, updated_at) \
         VALUES (?, ?, ?, 'file', ?, ?, ?, 0, 'pending', ?, ?, ?)",
    )
    .bind(&doc_id)
    .bind(&kb_id)
    .bind(&file_name)
    .bind(&relative_path)
    .bind(&mime_type)
    .bind(file_size)
    .bind(&hash)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Update document_count
    sqlx::query(
        "UPDATE knowledge_bases SET document_count = document_count + 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(&kb_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let doc = KbDocument {
        id: doc_id,
        kb_id: kb_id.clone(),
        name: file_name,
        source_type: "file".to_string(),
        source_path: Some(relative_path),
        source_url: None,
        mime_type,
        file_size,
        chunk_count: 0,
        indexing_status: "pending".to_string(),
        indexing_error: None,
        content_hash: Some(hash),
        created_at: now,
        updated_at: now,
    };

    // Auto-index in background
    if let Some(_store) = kb_store.as_ref() {
        let pool_c = pool.inner().clone();
        let app_handle_c = app_handle.clone();
        let kb_id_c = kb_id.clone();
        let doc_c = doc.clone();
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
        // Need KB data for chunking settings
        let kb_row = sqlx::query(
            "SELECT id, name, description, embedding_model, embedding_dimensions, chunking_strategy, \
             chunk_size, chunk_overlap, retrieval_top_k, retrieval_min_score, system_prompt, \
             version, status, document_count, total_chunks, created_at, updated_at \
             FROM knowledge_bases WHERE id = ?",
        )
        .bind(&kb_id_c)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        let kb = map_kb_row(kb_row);
        let provider = build_provider_for_kb(&app_handle, &kb).await?;
        let store_ref = KbVectorStore::new(
            app_data_dir.join("lancedb").to_string_lossy().as_ref()
        ).await.map_err(|e| format!("{}", e))?;
        tokio::spawn(async move {
            if let Err(e) = kb_indexer::index_document(
                &pool_c, &store_ref, &kb, &doc_c, &app_handle_c, &app_data_dir, provider.as_ref(),
            ).await {
                log::error!("[add_kb_document] Auto-index failed: {}", e);
            }
        });
    }

    Ok(doc)
}

#[tauri::command]
pub async fn remove_kb_document(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    kb_id: String,
    document_id: String,
) -> Result<(), String> {
    // Get chunk count before deletion for counter update
    let chunk_count: i64 = sqlx::query_scalar(
        "SELECT chunk_count FROM kb_documents WHERE id = ? AND kb_id = ?",
    )
    .bind(&document_id)
    .bind(&kb_id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Clean up vector and FTS indexes
    if let Some(store) = kb_store.as_ref() {
        let _ = kb_indexer::delete_document_index(pool.inner(), store, &kb_id, &document_id).await;
    }

    // Delete files from disk
    let doc_dir = get_kb_documents_dir(&app_handle)?
        .join(&kb_id)
        .join(&document_id);
    if doc_dir.exists() {
        std::fs::remove_dir_all(&doc_dir).map_err(|e| e.to_string())?;
    }

    // Delete document row (CASCADE deletes chunks)
    sqlx::query("DELETE FROM kb_documents WHERE id = ? AND kb_id = ?")
        .bind(&document_id)
        .bind(&kb_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Update counters
    let now = uni_common::now_unix_secs();
    sqlx::query(
        "UPDATE knowledge_bases SET document_count = MAX(document_count - 1, 0), \
         total_chunks = MAX(total_chunks - ?, 0), updated_at = ? WHERE id = ?",
    )
    .bind(chunk_count)
    .bind(now)
    .bind(&kb_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn add_kb_documents_bulk(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    kb_id: String,
    file_paths: Vec<String>,
) -> Result<Vec<KbDocument>, String> {
    let mut documents = Vec::new();

    for file_path in &file_paths {
        let source = std::path::Path::new(file_path);
        if !source.exists() {
            log::warn!("[add_kb_documents_bulk] File not found, skipping: {}", file_path);
            continue;
        }

        let file_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mime_type = detect_mime_type(extension).to_string();
        let doc_id = uni_common::generate_id();
        let now = uni_common::now_unix_secs();

        // Create target directory and copy file
        let doc_dir = get_kb_documents_dir(&app_handle)?
            .join(&kb_id)
            .join(&doc_id);
        std::fs::create_dir_all(&doc_dir).map_err(|e| e.to_string())?;

        let target = doc_dir.join(&file_name);
        std::fs::copy(source, &target).map_err(|e| e.to_string())?;

        let metadata = std::fs::metadata(&target).map_err(|e| e.to_string())?;
        let file_size = metadata.len() as i64;

        use sha2::{Digest, Sha256};
        let file_bytes = std::fs::read(&target).map_err(|e| e.to_string())?;
        let hash = format!("{:x}", Sha256::digest(&file_bytes));

        let relative_path = format!("kb_documents/{}/{}/{}", kb_id, doc_id, file_name);

        sqlx::query(
            "INSERT INTO kb_documents (id, kb_id, name, source_type, source_path, mime_type, \
             file_size, chunk_count, indexing_status, content_hash, created_at, updated_at) \
             VALUES (?, ?, ?, 'file', ?, ?, ?, 0, 'pending', ?, ?, ?)",
        )
        .bind(&doc_id)
        .bind(&kb_id)
        .bind(&file_name)
        .bind(&relative_path)
        .bind(&mime_type)
        .bind(file_size)
        .bind(&hash)
        .bind(now)
        .bind(now)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

        documents.push(KbDocument {
            id: doc_id,
            kb_id: kb_id.clone(),
            name: file_name,
            source_type: "file".to_string(),
            source_path: Some(relative_path),
            source_url: None,
            mime_type,
            file_size,
            chunk_count: 0,
            indexing_status: "pending".to_string(),
            indexing_error: None,
            content_hash: Some(hash),
            created_at: now,
            updated_at: now,
        });
    }

    // Update document_count in one shot
    let count = documents.len() as i64;
    if count > 0 {
        let now = uni_common::now_unix_secs();
        sqlx::query(
            "UPDATE knowledge_bases SET document_count = document_count + ?, updated_at = ? WHERE id = ?",
        )
        .bind(count)
        .bind(now)
        .bind(&kb_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    // Auto-index all new documents in background
    if !documents.is_empty() && kb_store.as_ref().is_some() {
        let pool_c = pool.inner().clone();
        let app_handle_c = app_handle.clone();
        let kb_id_c = kb_id.clone();
        let docs_c = documents.clone();
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
        let kb_row = sqlx::query(
            "SELECT id, name, description, embedding_model, embedding_dimensions, chunking_strategy, \
             chunk_size, chunk_overlap, retrieval_top_k, retrieval_min_score, system_prompt, \
             version, status, document_count, total_chunks, created_at, updated_at \
             FROM knowledge_bases WHERE id = ?",
        )
        .bind(&kb_id_c)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        let kb = map_kb_row(kb_row);
        let provider = build_provider_for_kb(&app_handle, &kb).await?;
        let store_ref = KbVectorStore::new(
            app_data_dir.join("lancedb").to_string_lossy().as_ref()
        ).await.map_err(|e| format!("{}", e))?;
        tokio::spawn(async move {
            for doc in &docs_c {
                if let Err(e) = kb_indexer::index_document(
                    &pool_c, &store_ref, &kb, doc, &app_handle_c, &app_data_dir, provider.as_ref(),
                ).await {
                    log::error!("[add_kb_documents_bulk] Auto-index failed for {}: {}", doc.id, e);
                }
            }
        });
    }

    Ok(documents)
}

#[tauri::command]
pub async fn get_kb_stats(pool: State<'_, Pool>, kb_id: String) -> Result<KbStats, String> {
    let row = sqlx::query(
        "SELECT \
         COUNT(*) as document_count, \
         COALESCE(SUM(chunk_count), 0) as total_chunks, \
         COALESCE(SUM(CASE WHEN indexing_status = 'indexed' THEN 1 ELSE 0 END), 0) as indexed_documents, \
         COALESCE(SUM(CASE WHEN indexing_status = 'pending' THEN 1 ELSE 0 END), 0) as pending_documents, \
         COALESCE(SUM(CASE WHEN indexing_status = 'failed' THEN 1 ELSE 0 END), 0) as failed_documents, \
         COALESCE(SUM(file_size), 0) as total_file_size \
         FROM kb_documents WHERE kb_id = ?",
    )
    .bind(&kb_id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(KbStats {
        document_count: row.get("document_count"),
        total_chunks: row.get("total_chunks"),
        indexed_documents: row.get("indexed_documents"),
        pending_documents: row.get("pending_documents"),
        failed_documents: row.get("failed_documents"),
        total_file_size: row.get("total_file_size"),
    })
}

// --- Indexing commands ---

#[tauri::command]
pub async fn index_kb_document(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    kb_id: String,
    document_id: String,
) -> Result<(), String> {
    let store = kb_store.as_ref().as_ref().ok_or("KB vector store not available")?;

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let kb = fetch_kb(pool.inner(), &kb_id).await?;
    let doc = fetch_doc(pool.inner(), &document_id).await?;

    // Delete existing index data for re-indexing
    let _ = kb_indexer::delete_document_index(pool.inner(), store, &kb_id, &document_id).await;

    // Reset chunk count on the document
    let now = uni_common::now_unix_secs();
    let old_chunks: i64 = doc.chunk_count;
    sqlx::query("UPDATE kb_documents SET chunk_count = 0, indexing_status = 'pending', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&document_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("UPDATE knowledge_bases SET total_chunks = MAX(total_chunks - ?, 0), updated_at = ? WHERE id = ?")
        .bind(old_chunks)
        .bind(now)
        .bind(&kb_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Spawn indexing in background
    let pool_c = pool.inner().clone();
    let app_handle_c = app_handle.clone();
    let provider = build_provider_for_kb(&app_handle, &kb).await?;
    let store_ref = KbVectorStore::new(
        app_data_dir.join("lancedb").to_string_lossy().as_ref()
    ).await.map_err(|e| format!("{}", e))?;
    tokio::spawn(async move {
        if let Err(e) = kb_indexer::index_document(
            &pool_c, &store_ref, &kb, &doc, &app_handle_c, &app_data_dir, provider.as_ref(),
        ).await {
            log::error!("[index_kb_document] failed: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn index_all_kb_documents(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    kb_id: String,
) -> Result<(), String> {
    let _store = kb_store.as_ref().as_ref().ok_or("KB vector store not available")?;
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let kb = fetch_kb(pool.inner(), &kb_id).await?;

    // Get all pending documents
    let rows = sqlx::query(
        "SELECT id, kb_id, name, source_type, source_path, source_url, mime_type, file_size, \
         chunk_count, indexing_status, indexing_error, content_hash, created_at, updated_at \
         FROM kb_documents WHERE kb_id = ? AND indexing_status = 'pending' ORDER BY created_at ASC",
    )
    .bind(&kb_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let docs: Vec<KbDocument> = rows.into_iter().map(map_doc_row).collect();
    if docs.is_empty() {
        return Ok(());
    }

    let pool_c = pool.inner().clone();
    let app_handle_c = app_handle.clone();
    let provider = build_provider_for_kb(&app_handle, &kb).await?;
    let store_ref = KbVectorStore::new(
        app_data_dir.join("lancedb").to_string_lossy().as_ref()
    ).await.map_err(|e| format!("{}", e))?;
    tokio::spawn(async move {
        for doc in &docs {
            if let Err(e) = kb_indexer::index_document(
                &pool_c, &store_ref, &kb, doc, &app_handle_c, &app_data_dir, provider.as_ref(),
            ).await {
                log::error!("[index_all_kb_documents] failed for {}: {}", doc.id, e);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn reindex_knowledge_base(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    kb_id: String,
) -> Result<(), String> {
    let store = kb_store.as_ref().as_ref().ok_or("KB vector store not available")?;
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;

    // Delete all existing index data
    let _ = kb_indexer::delete_kb_index(pool.inner(), store, &kb_id).await;

    // Delete all chunks from SQLite
    sqlx::query("DELETE FROM kb_chunks WHERE kb_id = ?")
        .bind(&kb_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Reset all documents to pending
    let now = uni_common::now_unix_secs();
    sqlx::query(
        "UPDATE kb_documents SET indexing_status = 'pending', chunk_count = 0, indexing_error = NULL, updated_at = ? WHERE kb_id = ?",
    )
    .bind(now)
    .bind(&kb_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Reset KB total_chunks and increment version
    sqlx::query(
        "UPDATE knowledge_bases SET total_chunks = 0, version = version + 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(&kb_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Fetch KB and all docs, then index
    let kb = fetch_kb(pool.inner(), &kb_id).await?;
    let rows = sqlx::query(
        "SELECT id, kb_id, name, source_type, source_path, source_url, mime_type, file_size, \
         chunk_count, indexing_status, indexing_error, content_hash, created_at, updated_at \
         FROM kb_documents WHERE kb_id = ? ORDER BY created_at ASC",
    )
    .bind(&kb_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    let docs: Vec<KbDocument> = rows.into_iter().map(map_doc_row).collect();

    let pool_c = pool.inner().clone();
    let app_handle_c = app_handle.clone();
    let provider = build_provider_for_kb(&app_handle, &kb).await?;
    let store_ref = KbVectorStore::new(
        app_data_dir.join("lancedb").to_string_lossy().as_ref()
    ).await.map_err(|e| format!("{}", e))?;
    tokio::spawn(async move {
        for doc in &docs {
            if let Err(e) = kb_indexer::index_document(
                &pool_c, &store_ref, &kb, doc, &app_handle_c, &app_data_dir, provider.as_ref(),
            ).await {
                log::error!("[reindex_knowledge_base] failed for {}: {}", doc.id, e);
            }
        }
    });

    Ok(())
}

// --- Search & RAG commands ---

#[tauri::command]
pub async fn search_knowledge_base(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    kb_id: String,
    query: String,
    top_k: Option<i64>,
) -> Result<Vec<KbSearchResultItem>, String> {
    let store = kb_store.as_ref().as_ref().ok_or("KB vector store not available")?;
    let kb = fetch_kb(pool.inner(), &kb_id).await?;
    let provider = build_provider_for_kb(&app_handle, &kb).await?;

    let config = KbSearchConfig {
        top_k: top_k.unwrap_or(kb.retrieval_top_k) as usize,
        min_score: kb.retrieval_min_score as f32,
        ..Default::default()
    };

    let emb_dims = Some(kb.embedding_dimensions as i32);
    kb_search::search_kb(
        pool.inner(),
        store,
        &kb_id,
        &query,
        &config,
        provider.as_ref(),
        emb_dims,
    )
    .await
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbDocSearchResult {
    pub chunk_id: String,
    pub kb_id: String,
    pub kb_name: String,
    pub document_id: String,
    pub document_name: String,
    pub content: String,
    pub chunk_index: i64,
    pub score: f32,
    pub heading_hierarchy: Option<String>,
}

#[tauri::command]
pub async fn search_all_knowledge_bases(
    pool: State<'_, Pool>,
    query: String,
    top_k: Option<i64>,
) -> Result<Vec<KbDocSearchResult>, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let limit = top_k.unwrap_or(20) as usize;

    // 1. FTS5 search across all KBs
    let fts_results = kb_fts::search_all_kb_fts(pool.inner(), &query, limit).await?;
    if fts_results.is_empty() {
        return Ok(Vec::new());
    }

    // 2. Normalize FTS ranks to 0..1
    let min_rank = fts_results.iter().map(|r| r.rank).fold(f64::INFINITY, f64::min);
    let max_rank = fts_results.iter().map(|r| r.rank).fold(f64::NEG_INFINITY, f64::max);
    let range = max_rank - min_rank;
    let scored: Vec<(String, f32)> = fts_results
        .iter()
        .map(|r| {
            let normalized = if range > 1e-9 {
                1.0 - (r.rank - min_rank) / range
            } else {
                1.0
            };
            (r.chunk_id.clone(), normalized.max(0.0).min(1.0) as f32)
        })
        .collect();

    // 3. Batch-load chunk details with document and KB names
    let chunk_ids: Vec<&str> = scored.iter().map(|(id, _)| id.as_str()).collect();
    let placeholders: String = chunk_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT kc.id, kc.content, kc.chunk_index, kc.document_id, kc.kb_id, kc.metadata, \
         kd.name AS document_name, kb.name AS kb_name \
         FROM kb_chunks kc \
         JOIN kb_documents kd ON kd.id = kc.document_id \
         JOIN knowledge_bases kb ON kb.id = kc.kb_id \
         WHERE kc.id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for id in &chunk_ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool.inner()).await.map_err(|e| e.to_string())?;

    // Build a map of chunk_id -> row data
    let mut row_map: HashMap<String, _> = HashMap::new();
    for row in &rows {
        let id: String = row.get("id");
        row_map.insert(id, row);
    }

    // 4. Build results in score order
    let mut results = Vec::with_capacity(scored.len());
    for (chunk_id, score) in &scored {
        if let Some(row) = row_map.get(chunk_id) {
            let metadata_str: String = row.get("metadata");
            let heading = serde_json::from_str::<serde_json::Value>(&metadata_str)
                .ok()
                .and_then(|v| v.get("heading_hierarchy").and_then(|h| h.as_str().map(String::from)));

            results.push(KbDocSearchResult {
                chunk_id: chunk_id.clone(),
                kb_id: row.get("kb_id"),
                kb_name: row.get("kb_name"),
                document_id: row.get("document_id"),
                document_name: row.get("document_name"),
                content: row.get("content"),
                chunk_index: row.get("chunk_index"),
                score: *score,
                heading_hierarchy: heading,
            });
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn attach_kb_to_chat(
    pool: State<'_, Pool>,
    chat_id: String,
    kb_id: String,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET kb_id = ? WHERE id = ?")
        .bind(&kb_id)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn detach_kb_from_chat(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET kb_id = NULL WHERE id = ?")
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_chat_kb(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<Option<KnowledgeBase>, String> {
    let row = sqlx::query(
        "SELECT kb.id, kb.name, kb.description, kb.embedding_model, kb.embedding_dimensions, \
         kb.chunking_strategy, kb.chunk_size, kb.chunk_overlap, kb.retrieval_top_k, \
         kb.retrieval_min_score, kb.reranker_type, kb.reranker_overfetch_factor, \
         kb.system_prompt, kb.version, kb.status, kb.document_count, \
         kb.total_chunks, kb.created_at, kb.updated_at \
         FROM knowledge_bases kb JOIN chats c ON c.kb_id = kb.id WHERE c.id = ?",
    )
    .bind(&chat_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(map_kb_row))
}

// --- Export / Import ---

#[derive(Debug, Serialize, Deserialize)]
struct KbManifest {
    format_version: u32,
    exported_at: i64,
    app_version: String,
    knowledge_base: KbManifestKb,
    documents: Vec<KbManifestDoc>,
    stats: KbManifestStats,
}

#[derive(Debug, Serialize, Deserialize)]
struct KbManifestKb {
    id: String,
    name: String,
    description: String,
    embedding_model: String,
    chunking_strategy: String,
    chunk_size: i64,
    chunk_overlap: i64,
    #[serde(default = "default_min_chunk_size")]
    min_chunk_size: i64,
    retrieval_top_k: i64,
    retrieval_min_score: f64,
    #[serde(default = "default_query_rewriting_enabled")]
    query_rewriting_enabled: bool,
    #[serde(default)]
    query_decomposition_enabled: bool,
    #[serde(default = "default_query_max_variants")]
    query_max_variants: i64,
    #[serde(default = "default_reranker_type")]
    reranker_type: String,
    #[serde(default = "default_reranker_overfetch_factor")]
    reranker_overfetch_factor: i64,
    #[serde(default = "default_context_token_budget")]
    context_token_budget: i64,
    #[serde(default = "default_true")]
    context_sentence_extraction: bool,
    #[serde(default = "default_true")]
    context_redundancy_removal: bool,
    version: i64,
}

fn default_min_chunk_size() -> i64 { 50 }
fn default_query_rewriting_enabled() -> bool { true }
fn default_query_max_variants() -> i64 { 3 }
fn default_reranker_type() -> String { "none".to_string() }
fn default_reranker_overfetch_factor() -> i64 { 4 }
fn default_context_token_budget() -> i64 { 4000 }
fn default_true() -> bool { true }

#[derive(Debug, Serialize, Deserialize)]
struct KbManifestDoc {
    id: String,
    name: String,
    mime_type: String,
    file_size: i64,
    file_path: String,
    content_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KbManifestStats {
    document_count: i64,
    total_chunks: i64,
}

#[tauri::command]
pub async fn export_knowledge_base(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    kb_id: String,
) -> Result<String, String> {
    let kb = fetch_kb(pool.inner(), &kb_id).await?;
    let docs = {
        let rows = sqlx::query(
            "SELECT id, kb_id, name, source_type, source_path, source_url, mime_type, file_size, \
             chunk_count, indexing_status, indexing_error, content_hash, created_at, updated_at \
             FROM kb_documents WHERE kb_id = ? ORDER BY created_at ASC",
        )
        .bind(&kb_id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        rows.into_iter().map(map_doc_row).collect::<Vec<_>>()
    };

    // Show save dialog
    let kb_name = kb.name.clone();
    let path = tokio::task::spawn_blocking({
        let app = app_handle.clone();
        move || {
            app.dialog()
                .file()
                .add_filter("UNI AI Knowledge Base", &["zip"])
                .set_file_name(&format!("{}.uni-kb.zip", kb_name))
                .blocking_save_file()
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Export cancelled".to_string())?;

    let zip_path = path.into_path().map_err(|e| e.to_string())?;

    // Build manifest
    let kb_docs_dir = get_kb_documents_dir(&app_handle)?;

    // Handle duplicate filenames
    let mut name_counts: HashMap<String, u32> = HashMap::new();
    let mut manifest_docs = Vec::new();
    let mut doc_file_mapping: Vec<(PathBuf, String)> = Vec::new(); // (source_path, zip_entry_name)

    for doc in &docs {
        let base_name = doc.name.clone();
        let count = name_counts.entry(base_name.clone()).or_insert(0);
        let zip_name = if *count == 0 {
            base_name.clone()
        } else {
            let stem = std::path::Path::new(&base_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&base_name);
            let ext = std::path::Path::new(&base_name)
                .extension()
                .and_then(|s| s.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();
            format!("{}_{}{}", stem, count, ext)
        };
        *count += 1;

        let source_path = kb_docs_dir.join(&kb_id).join(&doc.id).join(&doc.name);
        doc_file_mapping.push((source_path, zip_name.clone()));

        manifest_docs.push(KbManifestDoc {
            id: doc.id.clone(),
            name: doc.name.clone(),
            mime_type: doc.mime_type.clone(),
            file_size: doc.file_size,
            file_path: format!("documents/{}", zip_name),
            content_hash: doc.content_hash.clone(),
        });
    }

    let manifest = KbManifest {
        format_version: 1,
        exported_at: uni_common::now_unix_secs(),
        app_version: "0.1.0".to_string(),
        knowledge_base: KbManifestKb {
            id: kb.id.clone(),
            name: kb.name.clone(),
            description: kb.description.clone(),
            embedding_model: kb.embedding_model.clone(),
            chunking_strategy: kb.chunking_strategy.clone(),
            chunk_size: kb.chunk_size,
            chunk_overlap: kb.chunk_overlap,
            min_chunk_size: kb.min_chunk_size,
            retrieval_top_k: kb.retrieval_top_k,
            retrieval_min_score: kb.retrieval_min_score,
            query_rewriting_enabled: kb.query_rewriting_enabled,
            query_decomposition_enabled: kb.query_decomposition_enabled,
            query_max_variants: kb.query_max_variants,
            reranker_type: kb.reranker_type.clone(),
            reranker_overfetch_factor: kb.reranker_overfetch_factor,
            context_token_budget: kb.context_token_budget,
            context_sentence_extraction: kb.context_sentence_extraction,
            context_redundancy_removal: kb.context_redundancy_removal,
            version: kb.version,
        },
        documents: manifest_docs,
        stats: KbManifestStats {
            document_count: docs.len() as i64,
            total_chunks: kb.total_chunks,
        },
    };

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;

    // Create ZIP
    let file = std::fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Add manifest.json
    zip.start_file("manifest.json", opts).map_err(|e| e.to_string())?;
    zip.write_all(manifest_json.as_bytes()).map_err(|e| e.to_string())?;

    // Add documents
    for (source_path, zip_name) in &doc_file_mapping {
        if source_path.exists() {
            let content = std::fs::read(source_path).map_err(|e| e.to_string())?;
            zip.start_file(format!("documents/{}", zip_name), opts).map_err(|e| e.to_string())?;
            zip.write_all(&content).map_err(|e| e.to_string())?;
        }
    }

    // Add system prompt if set
    if !kb.system_prompt.is_empty() {
        zip.start_file("prompts/system_prompt.txt", opts).map_err(|e| e.to_string())?;
        zip.write_all(kb.system_prompt.as_bytes()).map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;

    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_knowledge_base(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    zip_path: String,
) -> Result<KnowledgeBase, String> {
    let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    // Read manifest
    let manifest: KbManifest = {
        let mut manifest_file = archive
            .by_name("manifest.json")
            .map_err(|_| "Invalid KB package: missing manifest.json".to_string())?;
        let mut buf = String::new();
        manifest_file.read_to_string(&mut buf).map_err(|e| e.to_string())?;
        serde_json::from_str(&buf).map_err(|e| format!("Invalid manifest: {}", e))?
    };

    if manifest.format_version != 1 {
        return Err(format!("Unsupported format version: {}", manifest.format_version));
    }

    // Read system prompt if present
    let system_prompt = {
        match archive.by_name("prompts/system_prompt.txt") {
            Ok(mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).map_err(|e| e.to_string())?;
                buf
            }
            Err(_) => String::new(),
        }
    };

    // Create new KB
    let new_kb_id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();
    let mkb = &manifest.knowledge_base;

    sqlx::query(
        "INSERT INTO knowledge_bases (id, name, description, embedding_model, chunking_strategy, \
         chunk_size, chunk_overlap, min_chunk_size, retrieval_top_k, retrieval_min_score, \
         query_rewriting_enabled, query_decomposition_enabled, query_max_variants, \
         reranker_type, reranker_overfetch_factor, \
         context_token_budget, context_sentence_extraction, context_redundancy_removal, \
         system_prompt, version, status, document_count, total_chunks, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 'active', 0, 0, ?, ?)",
    )
    .bind(&new_kb_id)
    .bind(&mkb.name)
    .bind(&mkb.description)
    .bind(&mkb.embedding_model)
    .bind(&mkb.chunking_strategy)
    .bind(mkb.chunk_size)
    .bind(mkb.chunk_overlap)
    .bind(mkb.min_chunk_size)
    .bind(mkb.retrieval_top_k)
    .bind(mkb.retrieval_min_score)
    .bind(mkb.query_rewriting_enabled)
    .bind(mkb.query_decomposition_enabled)
    .bind(mkb.query_max_variants)
    .bind(&mkb.reranker_type)
    .bind(mkb.reranker_overfetch_factor)
    .bind(mkb.context_token_budget)
    .bind(mkb.context_sentence_extraction)
    .bind(mkb.context_redundancy_removal)
    .bind(&system_prompt)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Extract documents
    let kb_docs_dir = get_kb_documents_dir(&app_handle)?;
    let mut imported_count: i64 = 0;

    for mdoc in &manifest.documents {
        let new_doc_id = uni_common::generate_id();
        let doc_dir = kb_docs_dir.join(&new_kb_id).join(&new_doc_id);
        std::fs::create_dir_all(&doc_dir).map_err(|e| e.to_string())?;

        // Extract file from ZIP
        let target_path = doc_dir.join(&mdoc.name);
        let file_size = match archive.by_name(&mdoc.file_path) {
            Ok(mut entry) => {
                let mut content = Vec::new();
                entry.read_to_end(&mut content).map_err(|e| e.to_string())?;
                let size = content.len() as i64;
                std::fs::write(&target_path, &content).map_err(|e| e.to_string())?;
                size
            }
            Err(_) => {
                log::warn!("[import_kb] Document not found in ZIP: {}", mdoc.file_path);
                continue;
            }
        };

        let relative_path = format!("kb_documents/{}/{}/{}", new_kb_id, new_doc_id, mdoc.name);

        // Compute hash of extracted file
        use sha2::{Digest, Sha256};
        let file_bytes = std::fs::read(&target_path).map_err(|e| e.to_string())?;
        let hash = format!("{:x}", Sha256::digest(&file_bytes));

        sqlx::query(
            "INSERT INTO kb_documents (id, kb_id, name, source_type, source_path, mime_type, \
             file_size, chunk_count, indexing_status, content_hash, created_at, updated_at) \
             VALUES (?, ?, ?, 'file', ?, ?, ?, 0, 'pending', ?, ?, ?)",
        )
        .bind(&new_doc_id)
        .bind(&new_kb_id)
        .bind(&mdoc.name)
        .bind(&relative_path)
        .bind(&mdoc.mime_type)
        .bind(file_size)
        .bind(&hash)
        .bind(now)
        .bind(now)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

        imported_count += 1;
    }

    // Update document_count
    sqlx::query(
        "UPDATE knowledge_bases SET document_count = ?, updated_at = ? WHERE id = ?",
    )
    .bind(imported_count)
    .bind(now)
    .bind(&new_kb_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(KnowledgeBase {
        id: new_kb_id,
        name: mkb.name.clone(),
        description: mkb.description.clone(),
        embedding_model: mkb.embedding_model.clone(),
        embedding_dimensions: uni_embedding::default_dimensions(match mkb.embedding_model.as_str() {
            "gemini" | "gemini-embedding" => "gemini",
            _ => "openai",
        }) as i64,
        chunking_strategy: mkb.chunking_strategy.clone(),
        chunk_size: mkb.chunk_size,
        chunk_overlap: mkb.chunk_overlap,
        min_chunk_size: mkb.min_chunk_size,
        retrieval_top_k: mkb.retrieval_top_k,
        retrieval_min_score: mkb.retrieval_min_score,
        query_rewriting_enabled: mkb.query_rewriting_enabled,
        query_decomposition_enabled: mkb.query_decomposition_enabled,
        query_max_variants: mkb.query_max_variants,
        reranker_type: mkb.reranker_type.clone(),
        reranker_overfetch_factor: mkb.reranker_overfetch_factor,
        context_token_budget: mkb.context_token_budget,
        context_sentence_extraction: mkb.context_sentence_extraction,
        context_redundancy_removal: mkb.context_redundancy_removal,
        system_prompt,
        version: 1,
        status: "active".to_string(),
        document_count: imported_count,
        total_chunks: 0,
        created_at: now,
        updated_at: now,
    })
}

// --- Helpers ---

/// Read embedding API key from settings store based on model_id
async fn read_embedding_api_key(app: &AppHandle, model_id: &str) -> Option<String> {
    let store = app.state::<Arc<JsonSettingsStore>>();
    match model_id {
        "openai" | "text-embedding-3-small" | "e5-small" => store
            .get("embedding.openai.api_key")
            .await
            .unwrap_or_default()
            .filter(|s| !s.is_empty()),
        "gemini" | "gemini-embedding" => store
            .get("embedding.gemini.api_key")
            .await
            .unwrap_or_default()
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

fn uni_model_id_for_kb(embedding_model: &str) -> &'static str {
    match embedding_model {
        "gemini" | "gemini-embedding" => "gemini",
        _ => "openai",
    }
}

/// Build embedding provider from a stored embedding model id (KB row or chat RAG).
pub async fn build_provider_for_embedding_model(
    app: &AppHandle,
    embedding_model: &str,
) -> Result<Box<dyn uni_embedding::EmbeddingProvider>, String> {
    let api_key = read_embedding_api_key(app, embedding_model).await.ok_or_else(|| {
        "Embedding API key not configured for this model (settings: OpenAI or Gemini key)".to_string()
    })?;
    let client = build_http_client(app, None).await?;
    uni_embedding::create_embedding_provider(
        uni_model_id_for_kb(embedding_model),
        &api_key,
        client,
    )
}

/// Build an embedding provider from KB settings
async fn build_provider_for_kb(
    app: &AppHandle,
    kb: &KnowledgeBase,
) -> Result<Box<dyn uni_embedding::EmbeddingProvider>, String> {
    build_provider_for_embedding_model(app, &kb.embedding_model).await
}

async fn fetch_kb(pool: &Pool, kb_id: &str) -> Result<KnowledgeBase, String> {
    let row = sqlx::query(
        "SELECT id, name, description, embedding_model, embedding_dimensions, chunking_strategy, chunk_size, chunk_overlap, \
         min_chunk_size, retrieval_top_k, retrieval_min_score, reranker_type, reranker_overfetch_factor, \
         context_token_budget, context_sentence_extraction, context_redundancy_removal, \
         system_prompt, version, status, document_count, \
         total_chunks, created_at, updated_at FROM knowledge_bases WHERE id = ?",
    )
    .bind(kb_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(map_kb_row(row))
}

async fn fetch_doc(pool: &Pool, doc_id: &str) -> Result<KbDocument, String> {
    let row = sqlx::query(
        "SELECT id, kb_id, name, source_type, source_path, source_url, mime_type, file_size, \
         chunk_count, indexing_status, indexing_error, content_hash, created_at, updated_at \
         FROM kb_documents WHERE id = ?",
    )
    .bind(doc_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(map_doc_row(row))
}

pub fn build_reranker_config_from_kb(
    reranker_type_str: &str,
    overfetch_factor: i64,
    cohere_key: &Option<String>,
    jina_key: &Option<String>,
) -> crate::services::kb_search_orchestrator::RerankerConfig {
    use uni_search::RerankerType;

    let reranker_type = match reranker_type_str {
        "cohere" => RerankerType::Cohere,
        "jina" => RerankerType::Jina,
        _ => RerankerType::None,
    };

    let api_key = match &reranker_type {
        RerankerType::Cohere => cohere_key.clone(),
        RerankerType::Jina => jina_key.clone(),
        RerankerType::None => None,
    };

    crate::services::kb_search_orchestrator::RerankerConfig {
        enabled: reranker_type != RerankerType::None && api_key.is_some(),
        reranker_type,
        api_key,
        overfetch_factor: overfetch_factor.max(2) as usize,
    }
}

pub fn build_optimization_config_from_kb(
    token_budget: i64,
    sentence_extraction: bool,
    redundancy_removal: bool,
) -> crate::services::context_optimizer::ContextOptimizationConfig {
    crate::services::context_optimizer::ContextOptimizationConfig {
        enabled: token_budget > 0 || sentence_extraction || redundancy_removal,
        token_budget: token_budget.max(0) as usize,
        sentence_extraction,
        redundancy_removal,
        ..Default::default()
    }
}

pub fn build_query_config_from_kb(
    kb_rewriting: bool,
    kb_decomposition: bool,
    kb_max_variants: i64,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
) -> crate::services::query_processor::QueryProcessingConfig {
    crate::services::query_processor::QueryProcessingConfig {
        multi_query_enabled: kb_rewriting,
        decomposition_enabled: kb_decomposition,
        max_variants: kb_max_variants.max(1) as usize,
        model,
        api_key,
        base_url,
    }
}

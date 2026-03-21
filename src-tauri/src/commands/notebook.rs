use sqlx::Row;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::models::knowledge_base::KbDocument;
use crate::models::notebook::{Notebook, NotebookWithStats};
use crate::services::kb_vector_store::KbVectorStore;
use crate::services::kb_indexer;
use crate::services::embedding_provider;
use crate::services::http_client::build_http_client;
use crate::services::web_content;

type Pool = sqlx::SqlitePool;
type KbStore = Arc<Option<KbVectorStore>>;

fn unix_now() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())
        .map(|d| d.as_secs() as i64)
}

fn get_kb_documents_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("kb_documents");
    Ok(dir)
}

async fn fetch_notebook(pool: &Pool, id: &str) -> Result<Notebook, String> {
    let row = sqlx::query_as::<_, Notebook>(
        "SELECT id, name, description, kb_id, created_at, updated_at FROM notebooks WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Notebook not found: {}", e))?;
    Ok(row)
}

fn extract_domain(url: &str) -> Option<String> {
    // Simple domain extraction without url crate
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_scheme.split('/').next()?;
    let host = host.split(':').next()?; // remove port
    if host.is_empty() { None } else { Some(host.to_string()) }
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
        _ => "text/plain",
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

fn map_kb_row(row: sqlx::sqlite::SqliteRow) -> crate::models::knowledge_base::KnowledgeBase {
    crate::models::knowledge_base::KnowledgeBase {
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

async fn fetch_kb(pool: &Pool, kb_id: &str) -> Result<crate::models::knowledge_base::KnowledgeBase, String> {
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

fn read_embedding_api_key(app: &AppHandle, model_id: &str) -> Option<String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").ok()?;
    let key = match model_id {
        "openai" | "text-embedding-3-small" => "embeddingOpenaiKey",
        "gemini" | "gemini-embedding" => "embeddingGeminiKey",
        _ => return None,
    };
    store
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

async fn build_provider_for_kb(
    app: &AppHandle,
    kb: &crate::models::knowledge_base::KnowledgeBase,
) -> Result<Box<dyn crate::services::embedding_provider::EmbeddingProvider>, String> {
    let api_key = read_embedding_api_key(app, &kb.embedding_model);
    let client = if kb.embedding_model != "e5-small" {
        Some(build_http_client(app, None).await?)
    } else {
        None
    };
    embedding_provider::create_embedding_provider(
        &kb.embedding_model,
        api_key.as_deref(),
        client,
    )
}

/// Spawn background indexing for a list of documents
async fn spawn_indexing(
    pool: &Pool,
    app_handle: &AppHandle,
    kb_id: &str,
    documents: Vec<KbDocument>,
) -> Result<(), String> {
    let pool_c = pool.clone();
    let app_handle_c = app_handle.clone();
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let kb = fetch_kb(pool, kb_id).await?;
    let provider = build_provider_for_kb(app_handle, &kb).await?;
    let store_ref = KbVectorStore::new(
        app_data_dir.join("lancedb").to_string_lossy().as_ref()
    ).await.map_err(|e| format!("{}", e))?;
    tokio::spawn(async move {
        for doc in &documents {
            if let Err(e) = kb_indexer::index_document(
                &pool_c, &store_ref, &kb, doc, &app_handle_c, &app_data_dir, provider.as_ref(),
            ).await {
                log::error!("[notebook] Auto-index failed for {}: {}", doc.id, e);
            }
        }
    });
    Ok(())
}

// --- Notebook CRUD ---

#[tauri::command]
pub async fn list_notebooks(pool: State<'_, Pool>) -> Result<Vec<NotebookWithStats>, String> {
    let rows = sqlx::query(
        "SELECT n.id, n.name, n.description, n.kb_id, n.created_at, n.updated_at, \
         COALESCE(kb.document_count, 0) as document_count, \
         COALESCE(kb.total_chunks, 0) as total_chunks \
         FROM notebooks n LEFT JOIN knowledge_bases kb ON n.kb_id = kb.id \
         ORDER BY n.updated_at DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| NotebookWithStats {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            kb_id: row.get("kb_id"),
            document_count: row.get("document_count"),
            total_chunks: row.get("total_chunks"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

#[tauri::command]
pub async fn get_notebook(pool: State<'_, Pool>, id: String) -> Result<NotebookWithStats, String> {
    let row = sqlx::query(
        "SELECT n.id, n.name, n.description, n.kb_id, n.created_at, n.updated_at, \
         COALESCE(kb.document_count, 0) as document_count, \
         COALESCE(kb.total_chunks, 0) as total_chunks \
         FROM notebooks n LEFT JOIN knowledge_bases kb ON n.kb_id = kb.id \
         WHERE n.id = ?",
    )
    .bind(&id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| format!("Notebook not found: {}", e))?;

    Ok(NotebookWithStats {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        kb_id: row.get("kb_id"),
        document_count: row.get("document_count"),
        total_chunks: row.get("total_chunks"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[tauri::command]
pub async fn create_notebook(
    pool: State<'_, Pool>,
    name: String,
    description: String,
) -> Result<NotebookWithStats, String> {
    let notebook_id = Uuid::new_v4().to_string();
    let kb_id = Uuid::new_v4().to_string();
    let now = unix_now()?;

    // Create KB under the hood
    sqlx::query(
        "INSERT INTO knowledge_bases (id, name, description, embedding_model, embedding_dimensions, \
         chunking_strategy, chunk_size, chunk_overlap, min_chunk_size, retrieval_top_k, retrieval_min_score, \
         query_rewriting_enabled, query_decomposition_enabled, query_max_variants, \
         reranker_type, reranker_overfetch_factor, context_token_budget, \
         context_sentence_extraction, context_redundancy_removal, \
         system_prompt, version, status, document_count, total_chunks, \
         notebook_id, created_at, updated_at) \
         VALUES (?, ?, '', 'e5-small', 384, 'tokens', 512, 50, 50, 5, 0.7, \
                 1, 0, 3, 'none', 4, 4000, 1, 1, '', 1, 'active', 0, 0, ?, ?, ?)",
    )
    .bind(&kb_id)
    .bind(format!("__notebook__{}", &name))
    .bind(&notebook_id)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Create notebook
    sqlx::query(
        "INSERT INTO notebooks (id, name, description, kb_id, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&notebook_id)
    .bind(&name)
    .bind(&description)
    .bind(&kb_id)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(NotebookWithStats {
        id: notebook_id,
        name,
        description,
        kb_id: Some(kb_id),
        document_count: 0,
        total_chunks: 0,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn update_notebook(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    description: String,
) -> Result<(), String> {
    let now = unix_now()?;
    sqlx::query(
        "UPDATE notebooks SET name = ?, description = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&description)
    .bind(now)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_notebook(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    id: String,
) -> Result<(), String> {
    let notebook = fetch_notebook(pool.inner(), &id).await?;

    if let Some(kb_id) = &notebook.kb_id {
        // Clean up vector and FTS indexes
        if let Some(store) = kb_store.as_ref() {
            let _ = kb_indexer::delete_kb_index(pool.inner(), store, kb_id).await;
        }

        // Delete files from disk
        let kb_dir = get_kb_documents_dir(&app_handle)?.join(kb_id);
        if kb_dir.exists() {
            std::fs::remove_dir_all(&kb_dir).map_err(|e| e.to_string())?;
        }

        // Delete KB row (CASCADE deletes documents and chunks)
        sqlx::query("DELETE FROM knowledge_bases WHERE id = ?")
            .bind(kb_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    // Delete notebook
    sqlx::query("DELETE FROM notebooks WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// --- Notebook document management ---

#[tauri::command]
pub async fn list_notebook_documents(
    pool: State<'_, Pool>,
    notebook_id: String,
) -> Result<Vec<KbDocument>, String> {
    let notebook = fetch_notebook(pool.inner(), &notebook_id).await?;
    let kb_id = notebook.kb_id.ok_or("Notebook has no KB")?;

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
pub async fn add_notebook_documents(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    notebook_id: String,
    file_paths: Vec<String>,
) -> Result<Vec<KbDocument>, String> {
    let notebook = fetch_notebook(pool.inner(), &notebook_id).await?;
    let kb_id = notebook.kb_id.ok_or("Notebook has no KB")?;
    let mut documents = Vec::new();

    for file_path in &file_paths {
        let source = std::path::Path::new(file_path);
        if !source.exists() {
            log::warn!("[add_notebook_documents] File not found, skipping: {}", file_path);
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
        let doc_id = Uuid::new_v4().to_string();
        let now = unix_now()?;

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

    // Update counters
    let count = documents.len() as i64;
    if count > 0 {
        let now = unix_now()?;
        sqlx::query(
            "UPDATE knowledge_bases SET document_count = document_count + ?, updated_at = ? WHERE id = ?",
        )
        .bind(count)
        .bind(now)
        .bind(&kb_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

        // Update notebook updated_at
        sqlx::query("UPDATE notebooks SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(&notebook_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

        if kb_store.as_ref().is_some() {
            spawn_indexing(pool.inner(), &app_handle, &kb_id, documents.clone()).await?;
        }
    }

    Ok(documents)
}

#[tauri::command]
pub async fn add_notebook_source(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    notebook_id: String,
    source_type: String,
    file_paths: Option<Vec<String>>,
    url: Option<String>,
    name: Option<String>,
    content: Option<String>,
) -> Result<Vec<KbDocument>, String> {
    match source_type.as_str() {
        "file" => {
            let paths = file_paths.ok_or("file_paths is required for source_type='file'")?;
            // Delegate to add_notebook_documents logic directly
            let notebook = fetch_notebook(pool.inner(), &notebook_id).await?;
            let kb_id = notebook.kb_id.ok_or("Notebook has no KB")?;
            let mut documents = Vec::new();

            for file_path in &paths {
                let source = std::path::Path::new(file_path);
                if !source.exists() {
                    log::warn!("[add_notebook_source] File not found, skipping: {}", file_path);
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
                let doc_id = Uuid::new_v4().to_string();
                let now = unix_now()?;

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

            let count = documents.len() as i64;
            if count > 0 {
                let now = unix_now()?;
                sqlx::query(
                    "UPDATE knowledge_bases SET document_count = document_count + ?, updated_at = ? WHERE id = ?",
                )
                .bind(count)
                .bind(now)
                .bind(&kb_id)
                .execute(pool.inner())
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query("UPDATE notebooks SET updated_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(&notebook_id)
                    .execute(pool.inner())
                    .await
                    .map_err(|e| e.to_string())?;

                if kb_store.as_ref().is_some() {
                    spawn_indexing(pool.inner(), &app_handle, &kb_id, documents.clone()).await?;
                }
            }

            Ok(documents)
        }
        "url" => {
            let url_str = url.ok_or("url is required for source_type='url'")?;
            let notebook = fetch_notebook(pool.inner(), &notebook_id).await?;
            let kb_id = notebook.kb_id.ok_or("Notebook has no KB")?;

            // Fetch web content
            let client = build_http_client(&app_handle, None).await?;
            let text = web_content::fetch_content(&client, &url_str, 50000).await?;
            if text.trim().is_empty() {
                return Err("No content extracted from URL".to_string());
            }

            let doc_id = Uuid::new_v4().to_string();
            let now = unix_now()?;

            // Extract domain for display name
            let doc_name = name.unwrap_or_else(|| {
                extract_domain(&url_str).unwrap_or_else(|| "Web page".to_string())
            });

            // Save content to file
            let doc_dir = get_kb_documents_dir(&app_handle)?
                .join(&kb_id)
                .join(&doc_id);
            std::fs::create_dir_all(&doc_dir).map_err(|e| e.to_string())?;

            let file_name = "page.md";
            let target = doc_dir.join(file_name);
            std::fs::write(&target, &text).map_err(|e| e.to_string())?;

            let file_size = text.len() as i64;
            use sha2::{Digest, Sha256};
            let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
            let relative_path = format!("kb_documents/{}/{}/{}", kb_id, doc_id, file_name);

            sqlx::query(
                "INSERT INTO kb_documents (id, kb_id, name, source_type, source_path, source_url, mime_type, \
                 file_size, chunk_count, indexing_status, content_hash, created_at, updated_at) \
                 VALUES (?, ?, ?, 'url', ?, ?, 'text/markdown', ?, 0, 'pending', ?, ?, ?)",
            )
            .bind(&doc_id)
            .bind(&kb_id)
            .bind(&doc_name)
            .bind(&relative_path)
            .bind(&url_str)
            .bind(file_size)
            .bind(&hash)
            .bind(now)
            .bind(now)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query(
                "UPDATE knowledge_bases SET document_count = document_count + 1, updated_at = ? WHERE id = ?",
            )
            .bind(now)
            .bind(&kb_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query("UPDATE notebooks SET updated_at = ? WHERE id = ?")
                .bind(now)
                .bind(&notebook_id)
                .execute(pool.inner())
                .await
                .map_err(|e| e.to_string())?;

            let doc = KbDocument {
                id: doc_id,
                kb_id: kb_id.clone(),
                name: doc_name,
                source_type: "url".to_string(),
                source_path: Some(relative_path),
                source_url: Some(url_str),
                mime_type: "text/markdown".to_string(),
                file_size,
                chunk_count: 0,
                indexing_status: "pending".to_string(),
                indexing_error: None,
                content_hash: Some(hash),
                created_at: now,
                updated_at: now,
            };

            if kb_store.as_ref().is_some() {
                spawn_indexing(pool.inner(), &app_handle, &kb_id, vec![doc.clone()]).await?;
            }

            Ok(vec![doc])
        }
        "youtube" => {
            let url_str = url.ok_or("url is required for source_type='youtube'")?;
            let notebook = fetch_notebook(pool.inner(), &notebook_id).await?;
            let kb_id = notebook.kb_id.ok_or("Notebook has no KB")?;

            let client = build_http_client(&app_handle, None).await?;
            let text = web_content::fetch_youtube_transcript(&client, &url_str).await?;
            if text.trim().is_empty() {
                return Err("No transcript extracted from YouTube video".to_string());
            }

            let doc_id = Uuid::new_v4().to_string();
            let now = unix_now()?;

            let doc_name = name.unwrap_or_else(|| format!("YouTube: {}", &url_str));

            let doc_dir = get_kb_documents_dir(&app_handle)?
                .join(&kb_id)
                .join(&doc_id);
            std::fs::create_dir_all(&doc_dir).map_err(|e| e.to_string())?;

            let file_name = "transcript.md";
            let target = doc_dir.join(file_name);
            std::fs::write(&target, &text).map_err(|e| e.to_string())?;

            let file_size = text.len() as i64;
            use sha2::{Digest, Sha256};
            let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
            let relative_path = format!("kb_documents/{}/{}/{}", kb_id, doc_id, file_name);

            sqlx::query(
                "INSERT INTO kb_documents (id, kb_id, name, source_type, source_path, source_url, mime_type, \
                 file_size, chunk_count, indexing_status, content_hash, created_at, updated_at) \
                 VALUES (?, ?, ?, 'youtube', ?, ?, 'text/markdown', ?, 0, 'pending', ?, ?, ?)",
            )
            .bind(&doc_id)
            .bind(&kb_id)
            .bind(&doc_name)
            .bind(&relative_path)
            .bind(&url_str)
            .bind(file_size)
            .bind(&hash)
            .bind(now)
            .bind(now)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query(
                "UPDATE knowledge_bases SET document_count = document_count + 1, updated_at = ? WHERE id = ?",
            )
            .bind(now)
            .bind(&kb_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query("UPDATE notebooks SET updated_at = ? WHERE id = ?")
                .bind(now)
                .bind(&notebook_id)
                .execute(pool.inner())
                .await
                .map_err(|e| e.to_string())?;

            let doc = KbDocument {
                id: doc_id,
                kb_id: kb_id.clone(),
                name: doc_name,
                source_type: "youtube".to_string(),
                source_path: Some(relative_path),
                source_url: Some(url_str),
                mime_type: "text/markdown".to_string(),
                file_size,
                chunk_count: 0,
                indexing_status: "pending".to_string(),
                indexing_error: None,
                content_hash: Some(hash),
                created_at: now,
                updated_at: now,
            };

            if kb_store.as_ref().is_some() {
                spawn_indexing(pool.inner(), &app_handle, &kb_id, vec![doc.clone()]).await?;
            }

            Ok(vec![doc])
        }
        "text" => {
            let text_name = name.ok_or("name is required for source_type='text'")?;
            let text_content = content.ok_or("content is required for source_type='text'")?;
            if text_content.trim().is_empty() {
                return Err("Content cannot be empty".to_string());
            }

            let notebook = fetch_notebook(pool.inner(), &notebook_id).await?;
            let kb_id = notebook.kb_id.ok_or("Notebook has no KB")?;

            let doc_id = Uuid::new_v4().to_string();
            let now = unix_now()?;

            let doc_dir = get_kb_documents_dir(&app_handle)?
                .join(&kb_id)
                .join(&doc_id);
            std::fs::create_dir_all(&doc_dir).map_err(|e| e.to_string())?;

            // Sanitize filename
            let safe_name = text_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            let file_name = format!("{}.md", safe_name);
            let target = doc_dir.join(&file_name);
            std::fs::write(&target, &text_content).map_err(|e| e.to_string())?;

            let file_size = text_content.len() as i64;
            use sha2::{Digest, Sha256};
            let hash = format!("{:x}", Sha256::digest(text_content.as_bytes()));
            let relative_path = format!("kb_documents/{}/{}/{}", kb_id, doc_id, file_name);

            sqlx::query(
                "INSERT INTO kb_documents (id, kb_id, name, source_type, source_path, mime_type, \
                 file_size, chunk_count, indexing_status, content_hash, created_at, updated_at) \
                 VALUES (?, ?, ?, 'text', ?, 'text/markdown', ?, 0, 'pending', ?, ?, ?)",
            )
            .bind(&doc_id)
            .bind(&kb_id)
            .bind(&text_name)
            .bind(&relative_path)
            .bind(file_size)
            .bind(&hash)
            .bind(now)
            .bind(now)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query(
                "UPDATE knowledge_bases SET document_count = document_count + 1, updated_at = ? WHERE id = ?",
            )
            .bind(now)
            .bind(&kb_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query("UPDATE notebooks SET updated_at = ? WHERE id = ?")
                .bind(now)
                .bind(&notebook_id)
                .execute(pool.inner())
                .await
                .map_err(|e| e.to_string())?;

            let doc = KbDocument {
                id: doc_id,
                kb_id: kb_id.clone(),
                name: text_name,
                source_type: "text".to_string(),
                source_path: Some(relative_path),
                source_url: None,
                mime_type: "text/markdown".to_string(),
                file_size,
                chunk_count: 0,
                indexing_status: "pending".to_string(),
                indexing_error: None,
                content_hash: Some(hash),
                created_at: now,
                updated_at: now,
            };

            if kb_store.as_ref().is_some() {
                spawn_indexing(pool.inner(), &app_handle, &kb_id, vec![doc.clone()]).await?;
            }

            Ok(vec![doc])
        }
        _ => Err(format!("Unknown source_type: {}", source_type)),
    }
}

#[tauri::command]
pub async fn remove_notebook_document(
    pool: State<'_, Pool>,
    kb_store: State<'_, KbStore>,
    app_handle: AppHandle,
    notebook_id: String,
    document_id: String,
) -> Result<(), String> {
    let notebook = fetch_notebook(pool.inner(), &notebook_id).await?;
    let kb_id = notebook.kb_id.ok_or("Notebook has no KB")?;

    // Get chunk count before deletion
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

    // Delete document row
    sqlx::query("DELETE FROM kb_documents WHERE id = ? AND kb_id = ?")
        .bind(&document_id)
        .bind(&kb_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Update counters
    let now = unix_now()?;
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

    sqlx::query("UPDATE notebooks SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&notebook_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

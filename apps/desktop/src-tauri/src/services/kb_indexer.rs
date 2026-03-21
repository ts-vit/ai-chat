// KB Indexer: orchestrates parsing → chunking → embedding → storage

use std::path::Path;
use sqlx::sqlite::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::models::knowledge_base::{KbDocument, KnowledgeBase};
use super::document_parser;
use uni_embedding::EmbeddingProvider;
use uni_search::chunk_document_enriched;
use super::kb_fts;
use super::kb_vector_store::{KbVectorEntry, KbVectorStore};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingProgress {
    pub kb_id: String,
    pub document_id: String,
    pub status: String,
    pub total_chunks: Option<usize>,
    pub error: Option<String>,
}

pub struct IndexingResult {
    pub chunk_count: usize,
    pub duration_ms: u64,
}

pub async fn index_document(
    pool: &SqlitePool,
    kb_vector_store: &KbVectorStore,
    kb: &KnowledgeBase,
    document: &KbDocument,
    app_handle: &AppHandle,
    app_data_dir: &Path,
    provider: &dyn EmbeddingProvider,
) -> Result<IndexingResult, String> {
    let start = std::time::Instant::now();
    let kb_id = &kb.id;
    let doc_id = &document.id;

    // 1. Set status to indexing
    update_doc_status(pool, doc_id, "indexing", None).await?;
    emit_progress(app_handle, kb_id, doc_id, "parsing", None, None);

    // 2. Resolve file path
    let source_path = document.source_path.as_deref().ok_or("No source path")?;
    let file_path = app_data_dir.join(source_path);
    let file_path_str = file_path.to_string_lossy().to_string();

    // 3. Extract text (use raw extraction for auto/html strategies to preserve HTML structure)
    let use_raw = matches!(kb.chunking_strategy.as_str(), "auto" | "html");
    let text = match if use_raw {
        document_parser::extract_text_raw(&file_path_str, &document.mime_type)
    } else {
        document_parser::extract_text(&file_path_str, &document.mime_type)
    } {
        Ok(t) => t,
        Err(e) => {
            let err = format!("Text extraction failed: {}", e);
            update_doc_status(pool, doc_id, "failed", Some(&err)).await?;
            emit_progress(app_handle, kb_id, doc_id, "failed", None, Some(&err));
            return Err(err);
        }
    };

    if text.trim().is_empty() {
        let err = "No text content extracted from document".to_string();
        update_doc_status(pool, doc_id, "failed", Some(&err)).await?;
        emit_progress(app_handle, kb_id, doc_id, "failed", None, Some(&err));
        return Err(err);
    }

    // 4. Chunk text (enriched: auto-detection, heading hierarchy, content_with_context)
    let chunks = chunk_document_enriched(
        &text,
        &kb.chunking_strategy,
        kb.chunk_size as usize,
        kb.chunk_overlap as usize,
        kb.min_chunk_size as usize,
        &document.name,
        &document.mime_type,
    );

    if chunks.is_empty() {
        let err = "Chunking produced no chunks".to_string();
        update_doc_status(pool, doc_id, "failed", Some(&err)).await?;
        emit_progress(app_handle, kb_id, doc_id, "failed", None, Some(&err));
        return Err(err);
    }

    let chunk_count = chunks.len();
    emit_progress(app_handle, kb_id, doc_id, "embedding", Some(chunk_count), None);

    // 5. Generate embeddings via provider (use content_with_context for richer embeddings)
    let chunk_texts: Vec<String> = chunks.iter().map(|c| {
        c.content_with_context.as_ref().unwrap_or(&c.content).clone()
    }).collect();
    let embeddings = provider.embed_documents(&chunk_texts).await?;

    emit_progress(app_handle, kb_id, doc_id, "storing", Some(chunk_count), None);

    // 6. Store chunks in SQLite
    let now = uni_common::now_unix_secs();

    let mut chunk_ids: Vec<String> = Vec::with_capacity(chunk_count);
    let mut fts_entries: Vec<(String, String, String, String)> = Vec::with_capacity(chunk_count);
    let mut vector_entries: Vec<KbVectorEntry> = Vec::with_capacity(chunk_count);

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_id = uni_common::generate_id();

        sqlx::query(
            "INSERT INTO kb_chunks (id, kb_id, document_id, content, chunk_index, start_offset, end_offset, metadata, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&chunk_id)
        .bind(kb_id)
        .bind(doc_id)
        .bind(&chunk.content)
        .bind(chunk.index as i64)
        .bind(chunk.start_offset as i64)
        .bind(chunk.end_offset as i64)
        .bind(chunk.metadata.to_string())
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to insert chunk: {}", e))?;

        fts_entries.push((
            chunk_id.clone(),
            kb_id.clone(),
            doc_id.clone(),
            chunk.content.clone(),
        ));

        vector_entries.push(KbVectorEntry {
            id: chunk_id.clone(),
            document_id: doc_id.clone(),
            chunk_index: chunk.index as i32,
            vector: embeddings[i].clone(),
        });

        chunk_ids.push(chunk_id);
    }

    // 7. Store in LanceDB
    let dims = Some(provider.dimensions() as i32);
    if let Err(e) = kb_vector_store.add_chunks(kb_id, vector_entries, dims).await {
        // Cleanup SQLite chunks on vector store failure
        for cid in &chunk_ids {
            let _ = sqlx::query("DELETE FROM kb_chunks WHERE id = ?")
                .bind(cid)
                .execute(pool)
                .await;
        }
        let err = format!("Vector store failed: {}", e);
        update_doc_status(pool, doc_id, "failed", Some(&err)).await?;
        emit_progress(app_handle, kb_id, doc_id, "failed", None, Some(&err));
        return Err(err);
    }

    // 8. Index in FTS5
    if let Err(e) = kb_fts::index_kb_chunks(pool, &fts_entries).await {
        log::warn!("FTS indexing failed (non-fatal): {}", e);
    }

    // 9. Update document status
    sqlx::query(
        "UPDATE kb_documents SET indexing_status = 'indexed', chunk_count = ?, indexing_error = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(chunk_count as i64)
    .bind(now)
    .bind(doc_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update document status: {}", e))?;

    // 10. Update KB total_chunks
    sqlx::query(
        "UPDATE knowledge_bases SET total_chunks = total_chunks + ?, updated_at = ? WHERE id = ?",
    )
    .bind(chunk_count as i64)
    .bind(now)
    .bind(kb_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update KB chunks count: {}", e))?;

    // 11. Emit done
    emit_progress(app_handle, kb_id, doc_id, "done", Some(chunk_count), None);

    let duration_ms = start.elapsed().as_millis() as u64;
    log::info!(
        "[kb_indexer] Indexed doc {} in KB {}: {} chunks in {}ms",
        doc_id, kb_id, chunk_count, duration_ms
    );

    Ok(IndexingResult {
        chunk_count,
        duration_ms,
    })
}

/// Delete all indexing data for a document (chunks, vectors, FTS).
pub async fn delete_document_index(
    pool: &SqlitePool,
    kb_vector_store: &KbVectorStore,
    kb_id: &str,
    document_id: &str,
) -> Result<(), String> {
    // Delete from LanceDB
    let _ = kb_vector_store.delete_document_chunks(kb_id, document_id).await;
    // Delete from FTS
    let _ = kb_fts::delete_kb_document_fts(pool, document_id).await;
    // Delete from SQLite chunks table (CASCADE would handle this too, but be explicit)
    sqlx::query("DELETE FROM kb_chunks WHERE document_id = ?")
        .bind(document_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to delete chunks: {}", e))?;
    Ok(())
}

/// Delete all indexing data for a KB.
pub async fn delete_kb_index(
    pool: &SqlitePool,
    kb_vector_store: &KbVectorStore,
    kb_id: &str,
) -> Result<(), String> {
    let _ = kb_vector_store.delete_kb(kb_id).await;
    let _ = kb_fts::delete_kb_fts(pool, kb_id).await;
    Ok(())
}

async fn update_doc_status(
    pool: &SqlitePool,
    doc_id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), String> {
    let now = uni_common::now_unix_secs();
    sqlx::query(
        "UPDATE kb_documents SET indexing_status = ?, indexing_error = ?, updated_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(error)
    .bind(now)
    .bind(doc_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update doc status: {}", e))?;
    Ok(())
}

fn emit_progress(
    app_handle: &AppHandle,
    kb_id: &str,
    doc_id: &str,
    status: &str,
    total_chunks: Option<usize>,
    error: Option<&str>,
) {
    let _ = app_handle.emit(
        "kb-indexing-progress",
        IndexingProgress {
            kb_id: kb_id.to_string(),
            document_id: doc_id.to_string(),
            status: status.to_string(),
            total_chunks,
            error: error.map(|e| e.to_string()),
        },
    );
}

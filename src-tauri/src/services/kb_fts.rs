// KB FTS5 full-text search for knowledge base chunks

use sqlx::sqlite::SqlitePool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct KbFtsResult {
    pub chunk_id: String,
    pub rank: f64,
}

/// Create the KB FTS5 virtual table if it doesn't exist.
pub async fn ensure_kb_fts_table(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE VIRTUAL TABLE IF NOT EXISTS kb_chunks_fts USING fts5(chunk_id, kb_id, document_id, content, tokenize='unicode61')",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create kb_chunks_fts: {}", e))?;
    Ok(())
}

/// Index chunks into FTS5.
/// Each tuple: (chunk_id, kb_id, document_id, content)
pub async fn index_kb_chunks(
    pool: &SqlitePool,
    chunks: &[(String, String, String, String)],
) -> Result<(), String> {
    for (chunk_id, kb_id, document_id, content) in chunks {
        sqlx::query(
            "INSERT INTO kb_chunks_fts(chunk_id, kb_id, document_id, content) VALUES(?, ?, ?, ?)",
        )
        .bind(chunk_id)
        .bind(kb_id)
        .bind(document_id)
        .bind(content)
        .execute(pool)
        .await
        .map_err(|e| format!("FTS index error: {}", e))?;
    }
    Ok(())
}

/// Search KB chunks via FTS5.
pub async fn search_kb_fts(
    pool: &SqlitePool,
    kb_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<KbFtsResult>, String> {
    let q = fts_escape_query(query);
    let rows = sqlx::query(
        "SELECT chunk_id, bm25(kb_chunks_fts) AS rank \
         FROM kb_chunks_fts \
         WHERE kb_id = ? AND kb_chunks_fts MATCH ? \
         ORDER BY bm25(kb_chunks_fts) \
         LIMIT ?",
    )
    .bind(kb_id)
    .bind(&q)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("FTS search error: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| KbFtsResult {
            chunk_id: row.get("chunk_id"),
            rank: row.get::<f64, _>("rank"),
        })
        .collect())
}

/// Delete all FTS entries for a document.
pub async fn delete_kb_document_fts(pool: &SqlitePool, document_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM kb_chunks_fts WHERE document_id = ?")
        .bind(document_id)
        .execute(pool)
        .await
        .map_err(|e| format!("FTS delete error: {}", e))?;
    Ok(())
}

/// Delete all FTS entries for a KB.
pub async fn delete_kb_fts(pool: &SqlitePool, kb_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM kb_chunks_fts WHERE kb_id = ?")
        .bind(kb_id)
        .execute(pool)
        .await
        .map_err(|e| format!("FTS delete error: {}", e))?;
    Ok(())
}

/// Search KB chunks via FTS5 across ALL knowledge bases (no kb_id filter).
pub async fn search_all_kb_fts(
    pool: &SqlitePool,
    query: &str,
    limit: usize,
) -> Result<Vec<KbFtsResult>, String> {
    let q = fts_escape_query(query);
    let rows = sqlx::query(
        "SELECT chunk_id, bm25(kb_chunks_fts) AS rank \
         FROM kb_chunks_fts \
         WHERE kb_chunks_fts MATCH ? \
         ORDER BY bm25(kb_chunks_fts) \
         LIMIT ?",
    )
    .bind(&q)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("FTS search error: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| KbFtsResult {
            chunk_id: row.get("chunk_id"),
            rank: row.get::<f64, _>("rank"),
        })
        .collect())
}

pub fn fts_escape_query(q: &str) -> String {
    let q = q.trim();
    if q.is_empty() {
        return "\"\"".to_string();
    }
    let escaped = q.replace('"', "\"\"");
    if !q.contains(char::is_whitespace) {
        escaped
    } else {
        format!("\"{}\"", escaped)
    }
}

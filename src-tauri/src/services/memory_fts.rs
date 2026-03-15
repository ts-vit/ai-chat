// FTS5 full-text search for agent memory

use sqlx::sqlite::SqlitePool;
use sqlx::Row;

pub async fn memory_fts_index(
    pool: &SqlitePool,
    memory_id: &str,
    content: &str,
) -> Result<(), String> {
    if content.trim().is_empty() {
        return Ok(());
    }
    let _ = sqlx::query("DELETE FROM agent_memory_fts WHERE memory_id = ?")
        .bind(memory_id)
        .execute(pool)
        .await;
    sqlx::query("INSERT INTO agent_memory_fts(memory_id, content) VALUES(?, ?)")
        .bind(memory_id)
        .bind(content)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn fts_escape_query(q: &str) -> String {
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

pub async fn memory_fts_search(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<(String, f64)>, String> {
    let q = fts_escape_query(query);
    let rows = sqlx::query(
        "SELECT memory_id, bm25(agent_memory_fts) AS rank
         FROM agent_memory_fts
         WHERE agent_memory_fts MATCH ?
         ORDER BY bm25(agent_memory_fts)
         LIMIT ?",
    )
    .bind(&q)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let results: Vec<(String, f64)> = rows
        .iter()
        .map(|row| {
            let id: String = row.try_get("memory_id").unwrap_or_default();
            let rank: f64 = row.try_get("rank").unwrap_or(0.0);
            (id, rank)
        })
        .collect();
    Ok(results)
}

pub async fn memory_fts_delete(pool: &SqlitePool, memory_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_memory_fts WHERE memory_id = ?")
        .bind(memory_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn memory_fts_delete_all(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_memory_fts")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

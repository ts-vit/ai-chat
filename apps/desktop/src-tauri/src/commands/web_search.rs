use tauri::AppHandle;

use crate::services::http_client::build_http_client;
use crate::services::web_content;
use crate::services::web_search::{self, SearchResult};

#[tauri::command]
pub async fn web_search(
    app: AppHandle,
    query: String,
    num_results: Option<u32>,
    provider: String,
    tavily_api_key: Option<String>,
    brave_api_key: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let count = num_results.unwrap_or(5);
    let client = build_http_client(&app, Some(std::time::Duration::from_secs(15))).await?;

    let mut results = match provider.as_str() {
        "tavily" => {
            let key = tavily_api_key
                .filter(|k| !k.trim().is_empty())
                .ok_or_else(|| "Tavily API key is required".to_string())?;
            web_search::search_tavily(&client, &key, &query, count).await?
        }
        "brave" => {
            let key = brave_api_key
                .filter(|k| !k.trim().is_empty())
                .ok_or_else(|| "Brave Search API key is required".to_string())?;
            web_search::search_brave(&client, &key, &query, count).await?
        }
        "uni" | "duckduckgo" => {
            web_search::search_uni(&query, count as usize, &client).await?
        }
        other => {
            return Err(format!("Unknown search provider: {}", other));
        }
    };

    let top_n = results.len().min(3);
    for i in 0..top_n {
        if results[i]
            .content
            .as_ref()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
        {
            continue;
        }
        match web_content::fetch_content(&client, &results[i].url, 2000).await {
            Ok(content) => {
                results[i].content = Some(content);
            }
            Err(e) => {
                eprintln!(
                    "[web_search] Failed to fetch content for {}: {}",
                    results[i].url, e
                );
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn update_message_web_sources(
    message_id: String,
    web_sources: Option<String>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<(), String> {
    sqlx::query("UPDATE messages SET web_sources = ? WHERE id = ?")
        .bind(&web_sources)
        .bind(&message_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_message_web_sources] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

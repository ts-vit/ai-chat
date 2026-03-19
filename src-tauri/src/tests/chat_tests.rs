#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_chat_minimal() {
        let pool = setup_test_db().await;
        let id = insert_chat_auto(&pool, "Test Chat", "chat").await;

        let row: (String, String, String, String) = sqlx::query_as(
            "SELECT id, title, mode, provider_id FROM chats WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, id);
        assert_eq!(row.1, "Test Chat");
        assert_eq!(row.2, "chat");
        assert_eq!(row.3, "openrouter");
    }

    #[tokio::test]
    async fn test_create_chat_with_all_params() {
        let pool = setup_test_db().await;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO chats (id, title, created_at, updated_at, system_prompt, provider_id, model, mode, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty, image_size, image_quality)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id).bind("Full Chat").bind(now).bind(now)
        .bind("You are helpful").bind("custom-1").bind("gpt-4").bind("assistant")
        .bind(0.7_f64).bind(4096_i64).bind(0.9_f64).bind(40_i64)
        .bind(0.5_f64).bind(0.3_f64).bind("1024x1024").bind("hd")
        .execute(&pool).await.unwrap();

        let row: (String, String, f64, i64, String) = sqlx::query_as(
            "SELECT mode, system_prompt, temperature, max_tokens, image_size FROM chats WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "assistant");
        assert_eq!(row.1, "You are helpful");
        assert!((row.2 - 0.7).abs() < 0.001);
        assert_eq!(row.3, 4096);
        assert_eq!(row.4, "1024x1024");
    }

    #[tokio::test]
    async fn test_get_chats_ordered_by_updated() {
        let pool = setup_test_db().await;
        let now = chrono::Utc::now().timestamp();

        for (i, title) in ["First", "Second", "Third"].iter().enumerate() {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO chats (id, title, created_at, updated_at, mode) VALUES (?, ?, ?, ?, 'chat')",
            )
            .bind(&id).bind(title).bind(now).bind(now + i as i64)
            .execute(&pool).await.unwrap();
        }

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT title FROM chats ORDER BY updated_at DESC",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows[0].0, "Third");
        assert_eq!(rows[1].0, "Second");
        assert_eq!(rows[2].0, "First");
    }

    #[tokio::test]
    async fn test_filter_chats_by_mode() {
        let pool = setup_test_db().await;
        insert_chat_auto(&pool, "Chat 1", "chat").await;
        insert_chat_auto(&pool, "Chat 2", "chat").await;
        insert_chat_auto(&pool, "Agent 1", "assistant").await;

        let chat_count = count_where(&pool, "chats", "mode = 'chat'").await;
        let agent_count = count_where(&pool, "chats", "mode = 'assistant'").await;

        assert_eq!(chat_count, 2);
        assert_eq!(agent_count, 1);
    }

    #[tokio::test]
    async fn test_update_chat_title() {
        let pool = setup_test_db().await;
        let id = insert_chat_auto(&pool, "Original", "chat").await;

        sqlx::query("UPDATE chats SET title = ?, updated_at = ? WHERE id = ?")
            .bind("Updated Title")
            .bind(chrono::Utc::now().timestamp() + 1)
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (String,) = sqlx::query_as("SELECT title FROM chats WHERE id = ?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row.0, "Updated Title");
    }

    #[tokio::test]
    async fn test_update_chat_model() {
        let pool = setup_test_db().await;
        let id = insert_chat_auto(&pool, "Test", "chat").await;

        sqlx::query("UPDATE chats SET model = ?, is_image_model = ? WHERE id = ?")
            .bind("dall-e-3")
            .bind(1)
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (String, i32) = sqlx::query_as(
            "SELECT model, is_image_model FROM chats WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "dall-e-3");
        assert_eq!(row.1, 1);
    }

    #[tokio::test]
    async fn test_update_chat_params() {
        let pool = setup_test_db().await;
        let id = insert_chat_auto(&pool, "Test", "chat").await;

        sqlx::query(
            "UPDATE chats SET temperature = ?, max_tokens = ?, top_p = ?, top_k = ?, frequency_penalty = ?, presence_penalty = ? WHERE id = ?",
        )
        .bind(0.8_f64).bind(2048_i64).bind(0.95_f64).bind(50_i64).bind(0.1_f64).bind(0.2_f64)
        .bind(&id)
        .execute(&pool).await.unwrap();

        let row: (f64, i64, f64, i64) = sqlx::query_as(
            "SELECT temperature, max_tokens, top_p, top_k FROM chats WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!((row.0 - 0.8).abs() < 0.001);
        assert_eq!(row.1, 2048);
        assert!((row.2 - 0.95).abs() < 0.001);
        assert_eq!(row.3, 50);
    }

    #[tokio::test]
    async fn test_chat_active_child_map_json() {
        let pool = setup_test_db().await;
        let id = insert_chat_auto(&pool, "Test", "chat").await;

        let map = r#"{"msg-1":"child-a","msg-2":"child-b"}"#;
        sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
            .bind(map)
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (String,) = sqlx::query_as(
            "SELECT active_child_map FROM chats WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&row.0).unwrap();
        assert_eq!(parsed["msg-1"], "child-a");
        assert_eq!(parsed["msg-2"], "child-b");
    }

    #[tokio::test]
    async fn test_chat_project_id_assignment() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Test", "assistant").await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;

        sqlx::query("UPDATE chats SET project_id = ? WHERE id = ?")
            .bind(&project_id)
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT project_id FROM chats WHERE id = ?",
        )
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, Some(project_id));
    }

    #[tokio::test]
    async fn test_chat_folder_id_assignment() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Test", "chat").await;
        let folder_id = insert_folder_auto(&pool, "Folder", "chat").await;

        sqlx::query("UPDATE chats SET folder_id = ? WHERE id = ?")
            .bind(&folder_id)
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT folder_id FROM chats WHERE id = ?",
        )
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, Some(folder_id));
    }

    #[tokio::test]
    async fn test_chat_kb_id_nullable() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Test", "chat").await;

        // Default is NULL
        let row: (Option<String>,) = sqlx::query_as("SELECT kb_id FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(row.0.is_none());

        // Set kb_id
        sqlx::query("UPDATE chats SET kb_id = ? WHERE id = ?")
            .bind("kb-123")
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as("SELECT kb_id FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0.as_deref(), Some("kb-123"));

        // Unset kb_id
        sqlx::query("UPDATE chats SET kb_id = NULL WHERE id = ?")
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as("SELECT kb_id FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(row.0.is_none());
    }

    #[tokio::test]
    async fn test_delete_chat() {
        let pool = setup_test_db().await;
        let id = insert_chat_auto(&pool, "Doomed", "chat").await;
        assert_eq!(count_rows(&pool, "chats").await, 1);

        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "chats").await, 0);
    }
}

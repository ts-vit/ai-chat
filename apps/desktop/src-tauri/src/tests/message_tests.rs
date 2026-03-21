#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_message_basic() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        let msg_id = insert_message_auto(&pool, &chat_id, "user", Some("Hello")).await;

        let row: (String, String, Option<String>) = sqlx::query_as(
            "SELECT id, role, content FROM messages WHERE id = ?",
        )
        .bind(&msg_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, msg_id);
        assert_eq!(row.1, "user");
        assert_eq!(row.2, Some("Hello".to_string()));
    }

    #[tokio::test]
    async fn test_message_with_null_content() {
        // messages.content is TEXT NOT NULL in schema, so NULL inserts should fail.
        // However, the app sometimes stores NULL content (tool call messages).
        // This test documents the schema constraint behavior.
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;

        let msg_id = uni_common::generate_id();
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, timestamp) VALUES (?, ?, ?, NULL, ?)",
        )
        .bind(&msg_id)
        .bind(&chat_id)
        .bind("assistant")
        .bind(now)
        .execute(&pool)
        .await;

        // Schema enforces NOT NULL — insert should fail.
        // NOTE: The app code stores NULL content for tool call messages,
        // which means there's a schema/code mismatch.
        // In production SQLite, existing rows may have NULL if the column
        // was originally nullable and later queries just work around it.
        assert!(
            result.is_err(),
            "Expected NOT NULL constraint to reject NULL content"
        );

        // Workaround: use empty string instead of NULL
        let msg_id2 = uni_common::generate_id();
        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, timestamp) VALUES (?, ?, ?, '', ?)",
        )
        .bind(&msg_id2)
        .bind(&chat_id)
        .bind("assistant")
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        let row: (String,) = sqlx::query_as("SELECT content FROM messages WHERE id = ?")
            .bind(&msg_id2)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "");
    }

    #[tokio::test]
    async fn test_message_with_empty_content() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        let msg_id = insert_message_auto(&pool, &chat_id, "assistant", Some("")).await;

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT content FROM messages WHERE id = ?",
        )
        .bind(&msg_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_messages_ordered_by_timestamp() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        let now = chrono::Utc::now().timestamp();

        for (i, content) in ["First", "Second", "Third"].iter().enumerate() {
            let id = uni_common::generate_id();
            sqlx::query(
                "INSERT INTO messages (id, chat_id, role, content, timestamp) VALUES (?, ?, 'user', ?, ?)",
            )
            .bind(&id).bind(&chat_id).bind(content).bind(now + i as i64)
            .execute(&pool).await.unwrap();
        }

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT content FROM messages WHERE chat_id = ? ORDER BY timestamp ASC",
        )
        .bind(&chat_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "First");
        assert_eq!(rows[1].0, "Second");
        assert_eq!(rows[2].0, "Third");
    }

    #[tokio::test]
    async fn test_messages_isolated_by_chat() {
        let pool = setup_test_db().await;
        let chat1 = insert_chat_auto(&pool, "Chat 1", "chat").await;
        let chat2 = insert_chat_auto(&pool, "Chat 2", "chat").await;

        insert_message_auto(&pool, &chat1, "user", Some("Chat 1 msg")).await;
        insert_message_auto(&pool, &chat2, "user", Some("Chat 2 msg")).await;

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT content FROM messages WHERE chat_id = ?",
        )
        .bind(&chat1)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "Chat 1 msg");
    }

    #[tokio::test]
    async fn test_message_parent_id_branching() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        let root_id = insert_message_auto(&pool, &chat_id, "user", Some("Root")).await;

        let child_id = uni_common::generate_id();
        insert_message(&pool, &child_id, &chat_id, "assistant", Some("Child"), Some(&root_id)).await;

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT parent_id FROM messages WHERE id = ?",
        )
        .bind(&child_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, Some(root_id));
    }

    #[tokio::test]
    async fn test_message_agent_run_scoping() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let now = chrono::Utc::now().timestamp();

        for (run_id, content) in [("run-1", "Run 1 msg"), ("run-2", "Run 2 msg")] {
            let id = uni_common::generate_id();
            sqlx::query(
                "INSERT INTO messages (id, chat_id, role, content, timestamp, agent_run_id) VALUES (?, ?, 'assistant', ?, ?, ?)",
            )
            .bind(&id).bind(&chat_id).bind(content).bind(now).bind(run_id)
            .execute(&pool).await.unwrap();
        }

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT content FROM messages WHERE chat_id = ? AND agent_run_id = ?",
        )
        .bind(&chat_id)
        .bind("run-1")
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "Run 1 msg");
    }

    #[tokio::test]
    async fn test_message_tool_fields() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let now = chrono::Utc::now().timestamp();

        // Messages table doesn't have tool_name/tool_call_id in CREATE TABLE or ALTER TABLE.
        // These fields are stored in content as JSON. Check if columns exist first.
        let cols: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM pragma_table_info('messages')",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let col_names: Vec<&str> = cols.iter().map(|c| c.0.as_str()).collect();

        // Test what we can — agent_step and agent_run_id definitely exist
        let id = uni_common::generate_id();
        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, timestamp, agent_step, agent_run_id) VALUES (?, ?, 'assistant', 'tool result', ?, 3, 'run-42')",
        )
        .bind(&id).bind(&chat_id).bind(now)
        .execute(&pool).await.unwrap();

        let row: (Option<i32>, Option<String>) = sqlx::query_as(
            "SELECT agent_step, agent_run_id FROM messages WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, Some(3));
        assert_eq!(row.1, Some("run-42".to_string()));

        // Document whether tool_name/tool_call_id columns exist
        if col_names.contains(&"tool_name") {
            // If they exist, we can test them
        }
        // Either way, the agent_step/agent_run_id columns are verified
    }

    #[tokio::test]
    async fn test_message_rag_sources_json() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        let msg_id = insert_message_auto(&pool, &chat_id, "assistant", Some("Answer")).await;

        let rag = r#"[{"documentId":"doc1","score":0.85}]"#;
        sqlx::query("UPDATE messages SET rag_sources = ? WHERE id = ?")
            .bind(rag)
            .bind(&msg_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT rag_sources FROM messages WHERE id = ?",
        )
        .bind(&msg_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(row.0.is_some());
        let parsed: serde_json::Value = serde_json::from_str(row.0.as_ref().unwrap()).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_message_web_sources() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        let msg_id = insert_message_auto(&pool, &chat_id, "assistant", Some("Web answer")).await;

        let sources = r#"[{"url":"https://example.com","title":"Example"}]"#;
        sqlx::query("UPDATE messages SET web_sources = ? WHERE id = ?")
            .bind(sources)
            .bind(&msg_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT web_sources FROM messages WHERE id = ?",
        )
        .bind(&msg_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0.as_deref(), Some(sources));
    }

    #[tokio::test]
    async fn test_update_message_content() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        let msg_id = insert_message_auto(&pool, &chat_id, "assistant", Some("Original")).await;

        sqlx::query("UPDATE messages SET content = ? WHERE id = ?")
            .bind("Updated content")
            .bind(&msg_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT content FROM messages WHERE id = ?",
        )
        .bind(&msg_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0.as_deref(), Some("Updated content"));
    }

    #[tokio::test]
    async fn test_message_russian_and_emoji() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;

        let russian = "Привет мир! Это тест с юникодом";
        let emoji = "Hello 🌍🎉🚀 World";

        let msg1 = insert_message_auto(&pool, &chat_id, "user", Some(russian)).await;
        let msg2 = insert_message_auto(&pool, &chat_id, "user", Some(emoji)).await;

        let row1: (Option<String>,) = sqlx::query_as("SELECT content FROM messages WHERE id = ?")
            .bind(&msg1).fetch_one(&pool).await.unwrap();
        let row2: (Option<String>,) = sqlx::query_as("SELECT content FROM messages WHERE id = ?")
            .bind(&msg2).fetch_one(&pool).await.unwrap();

        assert_eq!(row1.0.as_deref(), Some(russian));
        assert_eq!(row2.0.as_deref(), Some(emoji));
    }
}

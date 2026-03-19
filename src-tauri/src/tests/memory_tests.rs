#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_memory() {
        let pool = setup_test_db().await;
        let id = insert_memory_auto(&pool, "The user likes Rust", "preference").await;

        let row: (String, String, String, i32) = sqlx::query_as(
            "SELECT id, content, category, is_pinned FROM agent_memory WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, id);
        assert_eq!(row.1, "The user likes Rust");
        assert_eq!(row.2, "preference");
        assert_eq!(row.3, 0); // not pinned
    }

    #[tokio::test]
    async fn test_create_memory_with_project() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let mem_id = uuid::Uuid::new_v4().to_string();
        insert_memory(&pool, &mem_id, "project-specific fact", "fact", Some(&project_id)).await;

        let row: (Option<String>,) = sqlx::query_as(
            "SELECT project_id FROM agent_memory WHERE id = ?",
        )
        .bind(&mem_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0.as_deref(), Some(project_id.as_str()));
    }

    #[tokio::test]
    async fn test_list_memories_ordered() {
        let pool = setup_test_db().await;

        // Insert 3 memories, pin one
        let mem1 = insert_memory_auto(&pool, "Unpinned old", "fact").await;
        let mem2 = insert_memory_auto(&pool, "Unpinned new", "fact").await;
        let mem3 = insert_memory_auto(&pool, "Pinned", "fact").await;

        sqlx::query("UPDATE agent_memory SET is_pinned = 1 WHERE id = ?")
            .bind(&mem3).execute(&pool).await.unwrap();

        let rows: Vec<(String, i32)> = sqlx::query_as(
            "SELECT content, is_pinned FROM agent_memory ORDER BY is_pinned DESC, updated_at DESC",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "Pinned"); // pinned first
        assert_eq!(rows[0].1, 1);
    }

    #[tokio::test]
    async fn test_list_memories_by_category() {
        let pool = setup_test_db().await;
        insert_memory_auto(&pool, "fact 1", "fact").await;
        insert_memory_auto(&pool, "fact 2", "fact").await;
        insert_memory_auto(&pool, "pref 1", "preference").await;

        let fact_count = count_where(&pool, "agent_memory", "category = 'fact'").await;
        let pref_count = count_where(&pool, "agent_memory", "category = 'preference'").await;

        assert_eq!(fact_count, 2);
        assert_eq!(pref_count, 1);
    }

    #[tokio::test]
    async fn test_list_memories_by_project() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;

        insert_memory(&pool, &uuid::Uuid::new_v4().to_string(), "global", "fact", None).await;
        insert_memory(&pool, &uuid::Uuid::new_v4().to_string(), "project-specific", "fact", Some(&project_id)).await;

        let global_count = count_where(&pool, "agent_memory", "project_id IS NULL").await;
        let project_count = count_where(&pool, "agent_memory", &format!("project_id = '{}'", project_id)).await;

        assert_eq!(global_count, 1);
        assert_eq!(project_count, 1);
    }

    #[tokio::test]
    async fn test_update_memory_content() {
        let pool = setup_test_db().await;
        let id = insert_memory_auto(&pool, "Old fact", "fact").await;

        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE agent_memory SET content = ?, updated_at = ? WHERE id = ?")
            .bind("Updated fact").bind(now).bind(&id)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT content FROM agent_memory WHERE id = ?")
            .bind(&id).fetch_one(&pool).await.unwrap();

        assert_eq!(row.0, "Updated fact");
    }

    #[tokio::test]
    async fn test_pin_unpin_memory() {
        let pool = setup_test_db().await;
        let id = insert_memory_auto(&pool, "Fact", "fact").await;

        // Pin
        sqlx::query("UPDATE agent_memory SET is_pinned = 1, updated_at = ? WHERE id = ?")
            .bind(chrono::Utc::now().timestamp()).bind(&id)
            .execute(&pool).await.unwrap();

        let row: (i32,) = sqlx::query_as("SELECT is_pinned FROM agent_memory WHERE id = ?")
            .bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 1);

        // Unpin
        sqlx::query("UPDATE agent_memory SET is_pinned = 0, updated_at = ? WHERE id = ?")
            .bind(chrono::Utc::now().timestamp()).bind(&id)
            .execute(&pool).await.unwrap();

        let row: (i32,) = sqlx::query_as("SELECT is_pinned FROM agent_memory WHERE id = ?")
            .bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 0);
    }

    #[tokio::test]
    async fn test_delete_memory() {
        let pool = setup_test_db().await;
        let id = insert_memory_auto(&pool, "To delete", "fact").await;

        assert_eq!(count_rows(&pool, "agent_memory").await, 1);

        sqlx::query("DELETE FROM agent_memory WHERE id = ?")
            .bind(&id).execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "agent_memory").await, 0);
    }

    #[tokio::test]
    async fn test_fts_index_and_search() {
        let pool = setup_test_db().await;
        let id = insert_memory_auto(&pool, "Rust programming language is fast", "fact").await;

        // Index via the service function
        crate::services::memory_fts::memory_fts_index(&pool, &id, "Rust programming language is fast")
            .await
            .unwrap();

        // Search
        let results = crate::services::memory_fts::memory_fts_search(&pool, "Rust", 10)
            .await
            .unwrap();

        assert!(!results.is_empty(), "FTS should find 'Rust'");
        assert_eq!(results[0].0, id);
    }

    #[tokio::test]
    async fn test_delete_all_memories() {
        let pool = setup_test_db().await;
        insert_memory_auto(&pool, "fact 1", "fact").await;
        insert_memory_auto(&pool, "fact 2", "fact").await;
        insert_memory_auto(&pool, "fact 3", "fact").await;

        assert_eq!(count_rows(&pool, "agent_memory").await, 3);

        sqlx::query("DELETE FROM agent_memory").execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "agent_memory").await, 0);
    }
}

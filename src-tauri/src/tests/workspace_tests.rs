#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_artifact_chat_scoped() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let art_id = insert_artifact_auto(&pool, &chat_id, None, "notes.md", "Hello").await;

        let row: (String, String, Option<String>, String) = sqlx::query_as(
            "SELECT id, chat_id, project_id, name FROM workspace_artifacts WHERE id = ?",
        )
        .bind(&art_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, art_id);
        assert_eq!(row.1, chat_id);
        assert!(row.2.is_none());
        assert_eq!(row.3, "notes.md");
    }

    #[tokio::test]
    async fn test_create_artifact_project_scoped() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let art_id = insert_artifact_auto(&pool, &chat_id, Some(&project_id), "spec.md", "Spec content").await;

        let row: (Option<String>, String) = sqlx::query_as(
            "SELECT project_id, content FROM workspace_artifacts WHERE id = ?",
        )
        .bind(&art_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0.as_deref(), Some(project_id.as_str()));
        assert_eq!(row.1, "Spec content");
    }

    #[tokio::test]
    async fn test_read_artifact_chat_scoped() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        insert_artifact_auto(&pool, &chat_id, None, "readme.md", "Read me").await;

        let row: (String, String) = sqlx::query_as(
            "SELECT name, content FROM workspace_artifacts WHERE chat_id = ? AND project_id IS NULL AND name = ? LIMIT 1",
        )
        .bind(&chat_id)
        .bind("readme.md")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "readme.md");
        assert_eq!(row.1, "Read me");
    }

    #[tokio::test]
    async fn test_read_artifact_project_scoped() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        insert_artifact_auto(&pool, &chat_id, Some(&project_id), "spec.md", "Spec").await;

        let row: (String, String) = sqlx::query_as(
            "SELECT name, content FROM workspace_artifacts WHERE project_id = ? AND name = ? LIMIT 1",
        )
        .bind(&project_id)
        .bind("spec.md")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "spec.md");
        assert_eq!(row.1, "Spec");
    }

    #[tokio::test]
    async fn test_list_artifacts_chat_scoped() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        insert_artifact_auto(&pool, &chat_id, None, "a.md", "content a").await;
        insert_artifact_auto(&pool, &chat_id, None, "b.md", "content b").await;

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM workspace_artifacts WHERE chat_id = ? AND project_id IS NULL ORDER BY updated_at DESC",
        )
        .bind(&chat_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_list_artifacts_project_scoped() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat1 = insert_chat_auto(&pool, "Chat 1", "assistant").await;
        let chat2 = insert_chat_auto(&pool, "Chat 2", "assistant").await;

        insert_artifact_auto(&pool, &chat1, Some(&project_id), "from-chat1.md", "c1").await;
        insert_artifact_auto(&pool, &chat2, Some(&project_id), "from-chat2.md", "c2").await;

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM workspace_artifacts WHERE project_id = ? ORDER BY updated_at DESC",
        )
        .bind(&project_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_update_artifact_content() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let art_id = insert_artifact_auto(&pool, &chat_id, None, "doc.md", "v1").await;

        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE workspace_artifacts SET content = ?, updated_by = ?, updated_at = ? WHERE id = ?")
            .bind("v2").bind("agent").bind(now).bind(&art_id)
            .execute(&pool).await.unwrap();

        let row: (String, String) = sqlx::query_as(
            "SELECT content, updated_by FROM workspace_artifacts WHERE id = ?",
        )
        .bind(&art_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "v2");
        assert_eq!(row.1, "agent");
    }

    #[tokio::test]
    async fn test_delete_artifact() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let art_id = insert_artifact_auto(&pool, &chat_id, None, "tmp.md", "temp").await;

        assert_eq!(count_rows(&pool, "workspace_artifacts").await, 1);

        sqlx::query("DELETE FROM workspace_artifacts WHERE id = ?")
            .bind(&art_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "workspace_artifacts").await, 0);
    }

    #[tokio::test]
    async fn test_dual_scope_isolation() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;

        // Same name, different scopes
        insert_artifact_auto(&pool, &chat_id, None, "notes.md", "chat-scoped content").await;
        insert_artifact_auto(&pool, &chat_id, Some(&project_id), "notes.md", "project-scoped content").await;

        // Chat-scoped read
        let row: (String,) = sqlx::query_as(
            "SELECT content FROM workspace_artifacts WHERE chat_id = ? AND project_id IS NULL AND name = ?",
        )
        .bind(&chat_id).bind("notes.md")
        .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "chat-scoped content");

        // Project-scoped read
        let row: (String,) = sqlx::query_as(
            "SELECT content FROM workspace_artifacts WHERE project_id = ? AND name = ?",
        )
        .bind(&project_id).bind("notes.md")
        .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "project-scoped content");
    }

    #[tokio::test]
    async fn test_rename_artifact() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let art_id = insert_artifact_auto(&pool, &chat_id, None, "old.md", "content").await;

        sqlx::query("UPDATE workspace_artifacts SET name = ?, updated_at = ? WHERE id = ?")
            .bind("new.md").bind(chrono::Utc::now().timestamp()).bind(&art_id)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as(
            "SELECT name FROM workspace_artifacts WHERE id = ?",
        )
        .bind(&art_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "new.md");
    }
}

#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_project() {
        let pool = setup_test_db().await;
        let id = insert_project_auto(&pool, "My Project", "Build something").await;

        let row: (String, String, String, String) = sqlx::query_as(
            "SELECT id, name, goal, status FROM projects WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, id);
        assert_eq!(row.1, "My Project");
        assert_eq!(row.2, "Build something");
        assert_eq!(row.3, "active");
    }

    #[tokio::test]
    async fn test_get_project_by_id() {
        let pool = setup_test_db().await;
        let id = insert_project_auto(&pool, "Test", "Goal").await;

        let row: (String, String, String, i64, i64) = sqlx::query_as(
            "SELECT name, goal, status, created_at, updated_at FROM projects WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Test");
        assert!(row.3 > 0);
        assert!(row.4 > 0);
    }

    #[tokio::test]
    async fn test_project_summary_counts() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;

        // Add 2 chats
        let chat1 = insert_chat_auto(&pool, "Chat 1", "assistant").await;
        let chat2 = insert_chat_auto(&pool, "Chat 2", "assistant").await;
        sqlx::query("UPDATE chats SET project_id = ? WHERE id IN (?, ?)")
            .bind(&project_id).bind(&chat1).bind(&chat2)
            .execute(&pool).await.unwrap();

        // Add 1 artifact
        insert_artifact_auto(&pool, &chat1, Some(&project_id), "notes.md", "notes").await;

        // Add 1 memory
        insert_memory(&pool, &uni_common::generate_id(), "remember this", "fact", Some(&project_id)).await;

        // Add 1 plan
        let plan_id = uni_common::generate_id();
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO agent_plans (id, chat_id, goal, status, execution_mode, created_at, updated_at, project_id) VALUES (?, ?, 'plan goal', 'draft', 'manual', ?, ?, ?)",
        )
        .bind(&plan_id).bind(&chat1).bind(now).bind(now).bind(&project_id)
        .execute(&pool).await.unwrap();

        // Query summary with subqueries (like projects.rs does)
        let row: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT
                (SELECT COUNT(*) FROM chats WHERE project_id = p.id) as chat_count,
                (SELECT COUNT(*) FROM workspace_artifacts WHERE project_id = p.id) as artifact_count,
                (SELECT COUNT(*) FROM agent_memory WHERE project_id = p.id) as memory_count,
                (SELECT COUNT(*) FROM agent_plans WHERE project_id = p.id) as plan_count
             FROM projects p WHERE p.id = ?",
        )
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, 2); // chats
        assert_eq!(row.1, 1); // artifacts
        assert_eq!(row.2, 1); // memories
        assert_eq!(row.3, 1); // plans
    }

    #[tokio::test]
    async fn test_list_projects_ordered() {
        let pool = setup_test_db().await;
        let now = chrono::Utc::now().timestamp();

        for (i, name) in ["Alpha", "Beta", "Gamma"].iter().enumerate() {
            let id = uni_common::generate_id();
            sqlx::query(
                "INSERT INTO projects (id, name, goal, status, created_at, updated_at) VALUES (?, ?, '', 'active', ?, ?)",
            )
            .bind(&id).bind(name).bind(now).bind(now + i as i64)
            .execute(&pool).await.unwrap();
        }

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM projects ORDER BY updated_at DESC",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows[0].0, "Gamma");
        assert_eq!(rows[2].0, "Alpha");
    }

    #[tokio::test]
    async fn test_list_projects_by_status() {
        let pool = setup_test_db().await;
        insert_project_auto(&pool, "Active 1", "").await;
        insert_project_auto(&pool, "Active 2", "").await;

        let archived_id = insert_project_auto(&pool, "Archived", "").await;
        sqlx::query("UPDATE projects SET status = 'archived' WHERE id = ?")
            .bind(&archived_id).execute(&pool).await.unwrap();

        let active_count = count_where(&pool, "projects", "status = 'active'").await;
        let archived_count = count_where(&pool, "projects", "status = 'archived'").await;

        assert_eq!(active_count, 2);
        assert_eq!(archived_count, 1);
    }

    #[tokio::test]
    async fn test_update_project_name() {
        let pool = setup_test_db().await;
        let id = insert_project_auto(&pool, "Old Name", "Goal").await;

        sqlx::query("UPDATE projects SET name = ?, updated_at = ? WHERE id = ?")
            .bind("New Name")
            .bind(chrono::Utc::now().timestamp())
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (String, String) = sqlx::query_as(
            "SELECT name, goal FROM projects WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "New Name");
        assert_eq!(row.1, "Goal"); // unchanged
    }

    #[tokio::test]
    async fn test_update_project_status() {
        let pool = setup_test_db().await;
        let id = insert_project_auto(&pool, "Project", "Goal").await;

        for status in ["completed", "archived"] {
            sqlx::query("UPDATE projects SET status = ? WHERE id = ?")
                .bind(status).bind(&id)
                .execute(&pool).await.unwrap();

            let row: (String,) = sqlx::query_as("SELECT status FROM projects WHERE id = ?")
                .bind(&id).fetch_one(&pool).await.unwrap();
            assert_eq!(row.0, status);
        }
    }

    #[tokio::test]
    async fn test_assign_chat_to_project() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;

        sqlx::query("UPDATE chats SET project_id = ? WHERE id = ?")
            .bind(&project_id).bind(&chat_id)
            .execute(&pool).await.unwrap();

        let row: (Option<String>,) = sqlx::query_as("SELECT project_id FROM chats WHERE id = ?")
            .bind(&chat_id).fetch_one(&pool).await.unwrap();

        assert_eq!(row.0, Some(project_id));
    }

    #[tokio::test]
    async fn test_remove_chat_from_project() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;

        sqlx::query("UPDATE chats SET project_id = ? WHERE id = ?")
            .bind(&project_id).bind(&chat_id)
            .execute(&pool).await.unwrap();

        sqlx::query("UPDATE chats SET project_id = NULL WHERE id = ?")
            .bind(&chat_id)
            .execute(&pool).await.unwrap();

        let row: (Option<String>,) = sqlx::query_as("SELECT project_id FROM chats WHERE id = ?")
            .bind(&chat_id).fetch_one(&pool).await.unwrap();

        assert!(row.0.is_none());
    }

    #[tokio::test]
    async fn test_delete_project_nullifies_chats() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;

        sqlx::query("UPDATE chats SET project_id = ? WHERE id = ?")
            .bind(&project_id).bind(&chat_id)
            .execute(&pool).await.unwrap();

        // Manual cascade (like projects.rs does)
        sqlx::query("UPDATE chats SET project_id = NULL WHERE project_id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();

        // Chat still exists, project_id is NULL
        let row: (Option<String>,) = sqlx::query_as("SELECT project_id FROM chats WHERE id = ?")
            .bind(&chat_id).fetch_one(&pool).await.unwrap();
        assert!(row.0.is_none());
        assert_eq!(count_rows(&pool, "chats").await, 1);
    }

    #[tokio::test]
    async fn test_delete_project_removes_artifacts() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;

        insert_artifact_auto(&pool, &chat_id, Some(&project_id), "notes.md", "notes").await;
        insert_artifact_auto(&pool, &chat_id, Some(&project_id), "code.py", "code").await;

        assert_eq!(count_rows(&pool, "workspace_artifacts").await, 2);

        // Manual cascade
        sqlx::query("DELETE FROM workspace_artifacts WHERE project_id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "workspace_artifacts").await, 0);
    }

    #[tokio::test]
    async fn test_delete_project_removes_memory_and_plans() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;

        insert_memory(&pool, &uni_common::generate_id(), "fact 1", "fact", Some(&project_id)).await;

        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO agent_plans (id, chat_id, goal, status, execution_mode, created_at, updated_at, project_id) VALUES (?, ?, 'goal', 'draft', 'manual', ?, ?, ?)",
        )
        .bind(uni_common::generate_id()).bind(&chat_id).bind(now).bind(now).bind(&project_id)
        .execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "agent_memory").await, 1);
        assert_eq!(count_rows(&pool, "agent_plans").await, 1);

        // Manual cascade
        sqlx::query("DELETE FROM agent_memory WHERE project_id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM agent_plans WHERE project_id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();
        sqlx::query("UPDATE chats SET project_id = NULL WHERE project_id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(&project_id).execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "agent_memory").await, 0);
        assert_eq!(count_rows(&pool, "agent_plans").await, 0);
    }
}

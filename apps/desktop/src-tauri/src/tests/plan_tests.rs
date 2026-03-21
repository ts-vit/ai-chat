#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_plan_draft() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Build a feature").await;

        let row: (String, String, String, String) = sqlx::query_as(
            "SELECT id, goal, status, execution_mode FROM agent_plans WHERE id = ?",
        )
        .bind(&plan_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, plan_id);
        assert_eq!(row.1, "Build a feature");
        assert_eq!(row.2, "draft");
        assert_eq!(row.3, "manual");
    }

    #[tokio::test]
    async fn test_create_tasks_for_plan() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;

        insert_task_auto(&pool, &plan_id, "Setup", 0).await;
        insert_task_auto(&pool, &plan_id, "Implement", 1).await;
        insert_task_auto(&pool, &plan_id, "Test", 2).await;

        assert_eq!(count_rows(&pool, "agent_tasks").await, 3);
    }

    #[tokio::test]
    async fn test_task_dependencies_json() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;

        let task1 = insert_task_auto(&pool, &plan_id, "Task 1", 0).await;
        let task2_id = uni_common::generate_id();
        let deps = format!(r#"["{}"]"#, task1);
        insert_task(&pool, &task2_id, &plan_id, "Task 2", "pending", &deps, 1).await;

        let row: (String,) = sqlx::query_as(
            "SELECT dependencies FROM agent_tasks WHERE id = ?",
        )
        .bind(&task2_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let parsed: Vec<String> = serde_json::from_str(&row.0).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], task1);
    }

    #[tokio::test]
    async fn test_get_tasks_ordered() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;

        insert_task_auto(&pool, &plan_id, "Third", 2).await;
        insert_task_auto(&pool, &plan_id, "First", 0).await;
        insert_task_auto(&pool, &plan_id, "Second", 1).await;

        let rows: Vec<(String, i32)> = sqlx::query_as(
            "SELECT title, sort_order FROM agent_tasks WHERE plan_id = ? ORDER BY sort_order",
        )
        .bind(&plan_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows[0].0, "First");
        assert_eq!(rows[1].0, "Second");
        assert_eq!(rows[2].0, "Third");
    }

    #[tokio::test]
    async fn test_approve_plan() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;

        let result = sqlx::query(
            "UPDATE agent_plans SET status = 'approved', execution_mode = 'auto', updated_at = ? WHERE id = ? AND status = 'draft'",
        )
        .bind(chrono::Utc::now().timestamp())
        .bind(&plan_id)
        .execute(&pool).await.unwrap();

        assert_eq!(result.rows_affected(), 1);

        let row: (String, String) = sqlx::query_as(
            "SELECT status, execution_mode FROM agent_plans WHERE id = ?",
        )
        .bind(&plan_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "approved");
        assert_eq!(row.1, "auto");
    }

    #[tokio::test]
    async fn test_approve_non_draft_fails() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = uni_common::generate_id();
        insert_plan(&pool, &plan_id, &chat_id, "Goal", "running").await;

        let result = sqlx::query(
            "UPDATE agent_plans SET status = 'approved' WHERE id = ? AND status = 'draft'",
        )
        .bind(&plan_id)
        .execute(&pool).await.unwrap();

        assert_eq!(result.rows_affected(), 0);
    }

    #[tokio::test]
    async fn test_plan_status_transitions() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;

        for status in ["approved", "running", "completed"] {
            sqlx::query("UPDATE agent_plans SET status = ? WHERE id = ?")
                .bind(status).bind(&plan_id)
                .execute(&pool).await.unwrap();

            let row: (String,) = sqlx::query_as("SELECT status FROM agent_plans WHERE id = ?")
                .bind(&plan_id).fetch_one(&pool).await.unwrap();
            assert_eq!(row.0, status);
        }
    }

    #[tokio::test]
    async fn test_task_status_transitions() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;
        let task_id = insert_task_auto(&pool, &plan_id, "Task", 0).await;

        for status in ["running", "completed"] {
            sqlx::query("UPDATE agent_tasks SET status = ?, updated_at = ? WHERE id = ?")
                .bind(status).bind(chrono::Utc::now().timestamp()).bind(&task_id)
                .execute(&pool).await.unwrap();

            let row: (String,) = sqlx::query_as("SELECT status FROM agent_tasks WHERE id = ?")
                .bind(&task_id).fetch_one(&pool).await.unwrap();
            assert_eq!(row.0, status);
        }
    }

    #[tokio::test]
    async fn test_task_with_result() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;
        let task_id = insert_task_auto(&pool, &plan_id, "Task", 0).await;

        sqlx::query("UPDATE agent_tasks SET status = 'completed', result = ? WHERE id = ?")
            .bind("Successfully built the feature")
            .bind(&task_id)
            .execute(&pool).await.unwrap();

        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT status, result FROM agent_tasks WHERE id = ?",
        )
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "completed");
        assert_eq!(row.1.as_deref(), Some("Successfully built the feature"));
    }

    #[tokio::test]
    async fn test_plan_progress_counts() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;

        let t1 = insert_task_auto(&pool, &plan_id, "Task 1", 0).await;
        let t2 = insert_task_auto(&pool, &plan_id, "Task 2", 1).await;
        let _t3 = insert_task_auto(&pool, &plan_id, "Task 3", 2).await;

        // Complete 2 of 3
        sqlx::query("UPDATE agent_tasks SET status = 'completed' WHERE id = ?")
            .bind(&t1).execute(&pool).await.unwrap();
        sqlx::query("UPDATE agent_tasks SET status = 'completed' WHERE id = ?")
            .bind(&t2).execute(&pool).await.unwrap();

        let row: (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*) as total, COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0) as completed FROM agent_tasks WHERE plan_id = ?",
        )
        .bind(&plan_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, 3); // total
        assert_eq!(row.1, 2); // completed
    }

    #[tokio::test]
    async fn test_plan_replan_increment() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Goal").await;

        // Initial replan_count = 0
        let row: (i32,) = sqlx::query_as("SELECT replan_count FROM agent_plans WHERE id = ?")
            .bind(&plan_id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 0);

        // Increment
        sqlx::query("UPDATE agent_plans SET replan_count = replan_count + 1 WHERE id = ?")
            .bind(&plan_id).execute(&pool).await.unwrap();

        let row: (i32,) = sqlx::query_as("SELECT replan_count FROM agent_plans WHERE id = ?")
            .bind(&plan_id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 1);
    }

    #[tokio::test]
    async fn test_plan_project_scoping() {
        let pool = setup_test_db().await;
        let project_id = insert_project_auto(&pool, "Project", "Goal").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;

        let now = chrono::Utc::now().timestamp();
        let plan_id = uni_common::generate_id();
        sqlx::query(
            "INSERT INTO agent_plans (id, chat_id, goal, status, execution_mode, created_at, updated_at, project_id) VALUES (?, ?, 'plan goal', 'draft', 'manual', ?, ?, ?)",
        )
        .bind(&plan_id).bind(&chat_id).bind(now).bind(now).bind(&project_id)
        .execute(&pool).await.unwrap();

        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, goal FROM agent_plans WHERE project_id = ? ORDER BY created_at DESC",
        )
        .bind(&project_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "plan goal");
    }
}

#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_comparison_crud() {
        let pool = setup_test_db().await;
        let id = uni_common::generate_id();
        insert_comparison(&pool, &id).await;

        let row: (String, String, String) = sqlx::query_as(
            "SELECT title, left_model, right_model FROM comparisons WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Test comparison");
        assert_eq!(row.1, "model-a");
        assert_eq!(row.2, "model-b");

        // Delete
        sqlx::query("DELETE FROM comparisons WHERE id = ?")
            .bind(&id).execute(&pool).await.unwrap();
        assert_eq!(count_rows(&pool, "comparisons").await, 0);
    }

    #[tokio::test]
    async fn test_comparison_cascade_messages() {
        let pool = setup_test_db().await;
        let comp_id = uni_common::generate_id();
        insert_comparison(&pool, &comp_id).await;

        let msg1 = uni_common::generate_id();
        let msg2 = uni_common::generate_id();
        insert_comparison_message(&pool, &msg1, &comp_id, "user", None, "Question").await;
        insert_comparison_message(&pool, &msg2, &comp_id, "assistant", Some("left"), "Left answer").await;

        assert_eq!(count_rows(&pool, "comparison_messages").await, 2);

        sqlx::query("DELETE FROM comparisons WHERE id = ?")
            .bind(&comp_id).execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "comparison_messages").await, 0);
    }

    #[tokio::test]
    async fn test_snippets_with_category() {
        let pool = setup_test_db().await;
        let now = chrono::Utc::now().timestamp();

        let cat_id = uni_common::generate_id();
        sqlx::query("INSERT INTO categories (id, name, created_at) VALUES (?, ?, ?)")
            .bind(&cat_id).bind("Rust").bind(now)
            .execute(&pool).await.unwrap();

        let snip_id = uni_common::generate_id();
        sqlx::query("INSERT INTO snippets (id, name, content, category_id, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&snip_id).bind("Hello World").bind("fn main() {}").bind(&cat_id).bind(now)
            .execute(&pool).await.unwrap();

        let row: (String, String, String) = sqlx::query_as(
            "SELECT s.name, s.content, c.name FROM snippets s JOIN categories c ON s.category_id = c.id WHERE s.id = ?",
        )
        .bind(&snip_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Hello World");
        assert_eq!(row.1, "fn main() {}");
        assert_eq!(row.2, "Rust");
    }

    #[tokio::test]
    async fn test_custom_provider_crud() {
        let pool = setup_test_db().await;
        let now = chrono::Utc::now().timestamp();
        let id = uni_common::generate_id();

        sqlx::query("INSERT INTO custom_providers (id, name, base_url, created_at) VALUES (?, ?, ?, ?)")
            .bind(&id).bind("Local LLM").bind("http://localhost:8080").bind(now)
            .execute(&pool).await.unwrap();

        let row: (String, String, String) = sqlx::query_as(
            "SELECT name, base_url, api_key FROM custom_providers WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Local LLM");
        assert_eq!(row.1, "http://localhost:8080");
        assert_eq!(row.2, ""); // default

        // Update api_key
        sqlx::query("UPDATE custom_providers SET api_key = ? WHERE id = ?")
            .bind("sk-test").bind(&id)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT api_key FROM custom_providers WHERE id = ?")
            .bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "sk-test");

        // Delete
        sqlx::query("DELETE FROM custom_providers WHERE id = ?")
            .bind(&id).execute(&pool).await.unwrap();
        assert_eq!(count_rows(&pool, "custom_providers").await, 0);
    }

    #[tokio::test]
    async fn test_scheduled_task_crud() {
        let pool = setup_test_db().await;
        let id = uni_common::generate_id();
        insert_scheduled_task(&pool, &id, "Daily Report", "Generate a summary").await;

        let row: (String, String, String, i32, i64) = sqlx::query_as(
            "SELECT name, prompt, cron_expression, enabled, run_count FROM scheduled_tasks WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Daily Report");
        assert_eq!(row.1, "Generate a summary");
        assert_eq!(row.2, "0 * * * *");
        assert_eq!(row.3, 1); // enabled
        assert_eq!(row.4, 0); // run_count

        // Toggle enabled
        sqlx::query("UPDATE scheduled_tasks SET enabled = 0 WHERE id = ?")
            .bind(&id).execute(&pool).await.unwrap();

        let row: (i32,) = sqlx::query_as("SELECT enabled FROM scheduled_tasks WHERE id = ?")
            .bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 0);

        // Increment run_count
        sqlx::query("UPDATE scheduled_tasks SET run_count = run_count + 1 WHERE id = ?")
            .bind(&id).execute(&pool).await.unwrap();

        let row: (i64,) = sqlx::query_as("SELECT run_count FROM scheduled_tasks WHERE id = ?")
            .bind(&id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, 1);
    }

    #[tokio::test]
    async fn test_folder_crud() {
        let pool = setup_test_db().await;
        let id = insert_folder_auto(&pool, "My Folder", "chat").await;

        let row: (String, String) = sqlx::query_as(
            "SELECT name, mode FROM folders WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "My Folder");
        assert_eq!(row.1, "chat");

        // Update
        sqlx::query("UPDATE folders SET name = ?, color = ? WHERE id = ?")
            .bind("Renamed").bind("#D4854A").bind(&id)
            .execute(&pool).await.unwrap();

        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT name, color FROM folders WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Renamed");
        assert_eq!(row.1, Some("#D4854A".to_string()));
    }

    #[tokio::test]
    async fn test_folder_mode_filtering() {
        let pool = setup_test_db().await;
        insert_folder_auto(&pool, "Chat Folder", "chat").await;
        insert_folder_auto(&pool, "Agent Folder", "assistant").await;

        let chat_count = count_where(&pool, "folders", "mode = 'chat'").await;
        let agent_count = count_where(&pool, "folders", "mode = 'assistant'").await;

        assert_eq!(chat_count, 1);
        assert_eq!(agent_count, 1);
    }

    #[tokio::test]
    async fn test_chat_template_crud() {
        let pool = setup_test_db().await;
        let now = chrono::Utc::now().timestamp();
        let id = uni_common::generate_id();

        sqlx::query(
            "INSERT INTO chat_templates (id, name, icon, system_prompt, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id).bind("Code Review").bind("🔍").bind("You are a code reviewer").bind(0).bind(now)
        .execute(&pool).await.unwrap();

        let row: (String, String, String) = sqlx::query_as(
            "SELECT name, icon, system_prompt FROM chat_templates WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Code Review");
        assert_eq!(row.1, "🔍");
        assert_eq!(row.2, "You are a code reviewer");
    }

    #[tokio::test]
    async fn test_cost_ledger_insert() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO cost_ledger (timestamp, chat_id, model_id, provider, prompt_tokens, completion_tokens, cost, source) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(now).bind(&chat_id).bind("gpt-4").bind("openrouter").bind(100_i64).bind(200_i64).bind(0.003_f64).bind("agent")
        .execute(&pool).await.unwrap();

        let row: (i64, i64, f64, String, String) = sqlx::query_as(
            "SELECT prompt_tokens, completion_tokens, cost, source, model_id FROM cost_ledger WHERE chat_id = ?",
        )
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, 100);
        assert_eq!(row.1, 200);
        assert!((row.2 - 0.003).abs() < 0.0001);
        assert_eq!(row.3, "agent");
        assert_eq!(row.4, "gpt-4");
    }

    #[tokio::test]
    async fn test_agent_run_crud() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let run_id = insert_agent_run_auto(&pool, &chat_id).await;

        let row: (String, String, i32, i32) = sqlx::query_as(
            "SELECT id, status, iterations, max_iterations FROM agent_runs WHERE id = ?",
        )
        .bind(&run_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, run_id);
        assert_eq!(row.1, "running");
        assert_eq!(row.2, 0);
        assert_eq!(row.3, 25);

        // Update status and cost
        sqlx::query("UPDATE agent_runs SET status = 'completed', cost = ?, prompt_tokens = ?, completion_tokens = ?, finished_at = ? WHERE id = ?")
            .bind(0.05_f64).bind(500_i64).bind(300_i64).bind(chrono::Utc::now().timestamp()).bind(&run_id)
            .execute(&pool).await.unwrap();

        let row: (String, f64, i64) = sqlx::query_as(
            "SELECT status, cost, prompt_tokens FROM agent_runs WHERE id = ?",
        )
        .bind(&run_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "completed");
        assert!((row.1 - 0.05).abs() < 0.001);
        assert_eq!(row.2, 500);
    }

    #[tokio::test]
    async fn test_null_optional_fields() {
        let pool = setup_test_db().await;
        let id = insert_chat_auto(&pool, "Chat", "chat").await;

        let row: (Option<f64>, Option<i64>, Option<f64>, Option<i64>) = sqlx::query_as(
            "SELECT temperature, max_tokens, top_p, top_k FROM chats WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(row.0.is_none());
        assert!(row.1.is_none());
        assert!(row.2.is_none());
        assert!(row.3.is_none());
    }
}

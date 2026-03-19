#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_all_tables_created() {
        let pool = setup_test_db().await;
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();

        let expected = [
            "chats",
            "messages",
            "folders",
            "presets",
            "categories",
            "snippets",
            "custom_providers",
            "chat_templates",
            "mcp_servers",
            "image_styles",
            "comparisons",
            "comparison_messages",
            "prompt_library",
            "mode_settings",
            "agent_runs",
            "agent_memory",
            "agent_plans",
            "agent_tasks",
            "skills",
            "chat_skills",
            "model_catalog",
            "cost_ledger",
            "routing_rules",
            "workspace_artifacts",
            "projects",
            "scheduled_tasks",
            "knowledge_bases",
            "kb_documents",
            "kb_chunks",
            "fs_audit_log",
        ];
        for table in &expected {
            assert!(names.contains(table), "Missing table: {}", table);
        }
    }

    #[tokio::test]
    async fn test_chats_has_all_columns() {
        let pool = setup_test_db().await;
        let columns: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('chats')")
                .fetch_all(&pool)
                .await
                .unwrap();
        let col_names: Vec<&str> = columns.iter().map(|c| c.0.as_str()).collect();

        let expected = [
            "id", "title", "created_at", "updated_at",
            "system_prompt", "provider_id", "model", "folder_id",
            "is_image_model", "temperature", "max_tokens", "top_p", "top_k",
            "frequency_penalty", "presence_penalty", "image_size", "image_quality",
            "image_style", "image_n", "negative_prompt", "active_child_map",
            "branch_migrated", "mode", "agent_max_iterations", "agent_auto_mode",
            "auto_skill_detection", "project_id", "is_telegram_chat",
            "scheduled_task_id", "kb_id",
        ];
        for col in &expected {
            assert!(col_names.contains(col), "Missing column on chats: {}", col);
        }
    }

    #[tokio::test]
    async fn test_messages_has_all_columns() {
        let pool = setup_test_db().await;
        let columns: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('messages')")
                .fetch_all(&pool)
                .await
                .unwrap();
        let col_names: Vec<&str> = columns.iter().map(|c| c.0.as_str()).collect();

        let expected = [
            "id", "chat_id", "role", "content", "parent_id", "timestamp",
            "model", "prompt_tokens", "completion_tokens", "cost",
            "has_attachments", "fts_indexed", "web_sources",
            "agent_step", "agent_run_id", "rag_sources",
        ];
        for col in &expected {
            assert!(col_names.contains(col), "Missing column on messages: {}", col);
        }
    }

    #[tokio::test]
    async fn test_agent_runs_has_all_columns() {
        let pool = setup_test_db().await;
        let columns: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('agent_runs')")
                .fetch_all(&pool)
                .await
                .unwrap();
        let col_names: Vec<&str> = columns.iter().map(|c| c.0.as_str()).collect();

        let expected = [
            "id", "chat_id", "status", "iterations", "max_iterations",
            "started_at", "finished_at", "error",
            "prompt_tokens", "completion_tokens", "cost", "assigned_model",
            "parent_run_id", "orchestrator_chat_id", "spawn_config", "depth",
        ];
        for col in &expected {
            assert!(col_names.contains(col), "Missing column on agent_runs: {}", col);
        }
    }

    #[tokio::test]
    async fn test_fts_tables_created() {
        let pool = setup_test_db().await;
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%fts%'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();

        assert!(names.contains(&"messages_fts"), "Missing messages_fts");
        assert!(names.contains(&"agent_memory_fts"), "Missing agent_memory_fts");
        assert!(names.contains(&"kb_chunks_fts"), "Missing kb_chunks_fts");
    }
}

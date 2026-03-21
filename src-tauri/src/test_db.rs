#[cfg(test)]
pub mod test_helpers {
    use sqlx::sqlite::SqlitePool;

    /// Create an in-memory SQLite database with all migrations applied.
    pub async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory SQLite pool");

        // Critical: SQLite doesn't enforce FKs by default
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("Failed to enable foreign keys");

        // Run CREATE TABLE migrations
        for sql in crate::DB_MIGRATIONS {
            sqlx::query(sql)
                .execute(&pool)
                .await
                .unwrap_or_else(|e| panic!("Migration failed: {}\nSQL: {}", e, sql));
        }

        // Run ALTER TABLE migrations (silently ignore duplicates)
        for sql in crate::ALTER_QUERIES {
            let _ = sqlx::query(sql).execute(&pool).await;
        }

        // fs_audit_log (created outside DB_MIGRATIONS in lib.rs)
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS fs_audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                tool_name TEXT NOT NULL,
                path TEXT NOT NULL,
                result TEXT NOT NULL,
                details TEXT DEFAULT '',
                chat_id TEXT DEFAULT ''
            )",
        )
        .execute(&pool)
        .await;

        // kb_chunks_fts virtual table
        let _ = sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS kb_chunks_fts USING fts5(chunk_id, kb_id, document_id, content, tokenize='unicode61')",
        )
        .execute(&pool)
        .await;

        pool
    }

    fn now_secs() -> i64 {
        chrono::Utc::now().timestamp()
    }

    /// Insert a minimal chat, returns the generated id.
    pub async fn insert_chat(pool: &SqlitePool, id: &str, title: &str, mode: &str) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO chats (id, title, created_at, updated_at, system_prompt, provider_id, model, mode)
             VALUES (?, ?, ?, ?, '', 'openrouter', 'test-model', ?)",
        )
        .bind(id)
        .bind(title)
        .bind(now)
        .bind(now)
        .bind(mode)
        .execute(pool)
        .await
        .expect("insert_chat failed");
        id.to_string()
    }

    /// Insert a chat with auto-generated uuid, returns the id.
    pub async fn insert_chat_auto(pool: &SqlitePool, title: &str, mode: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_chat(pool, &id, title, mode).await
    }

    /// Insert a message. content can be None to test NULL content.
    pub async fn insert_message(
        pool: &SqlitePool,
        id: &str,
        chat_id: &str,
        role: &str,
        content: Option<&str>,
        parent_id: Option<&str>,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(chat_id)
        .bind(role)
        .bind(content)
        .bind(parent_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_message failed");
        id.to_string()
    }

    /// Insert a message with auto-generated uuid.
    pub async fn insert_message_auto(
        pool: &SqlitePool,
        chat_id: &str,
        role: &str,
        content: Option<&str>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_message(pool, &id, chat_id, role, content, None).await
    }

    /// Insert a project, returns the id.
    pub async fn insert_project(pool: &SqlitePool, id: &str, name: &str, goal: &str) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO projects (id, name, goal, status, created_at, updated_at)
             VALUES (?, ?, ?, 'active', ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(goal)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_project failed");
        id.to_string()
    }

    pub async fn insert_project_auto(pool: &SqlitePool, name: &str, goal: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_project(pool, &id, name, goal).await
    }

    /// Insert a folder, returns the id.
    pub async fn insert_folder(pool: &SqlitePool, id: &str, name: &str, mode: &str) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO folders (id, name, sort_order, created_at, mode)
             VALUES (?, ?, 0, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(now)
        .bind(mode)
        .execute(pool)
        .await
        .expect("insert_folder failed");
        id.to_string()
    }

    pub async fn insert_folder_auto(pool: &SqlitePool, name: &str, mode: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_folder(pool, &id, name, mode).await
    }

    /// Insert a skill, returns the id.
    pub async fn insert_skill(pool: &SqlitePool, id: &str, name: &str, is_builtin: bool) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO skills (id, name, description, icon, content, trigger_description, required_tools, is_builtin, enabled, sort_order, created_at, updated_at)
             VALUES (?, ?, '', 'code', 'test content', '', '[]', ?, 1, 0, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(is_builtin as i32)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_skill failed");
        id.to_string()
    }

    pub async fn insert_skill_auto(pool: &SqlitePool, name: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_skill(pool, &id, name, false).await
    }

    /// Insert an agent plan, returns the id.
    pub async fn insert_plan(
        pool: &SqlitePool,
        id: &str,
        chat_id: &str,
        goal: &str,
        status: &str,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO agent_plans (id, chat_id, goal, status, execution_mode, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'manual', ?, ?)",
        )
        .bind(id)
        .bind(chat_id)
        .bind(goal)
        .bind(status)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_plan failed");
        id.to_string()
    }

    pub async fn insert_plan_auto(pool: &SqlitePool, chat_id: &str, goal: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_plan(pool, &id, chat_id, goal, "draft").await
    }

    /// Insert an agent task, returns the id.
    pub async fn insert_task(
        pool: &SqlitePool,
        id: &str,
        plan_id: &str,
        title: &str,
        status: &str,
        deps_json: &str,
        sort_order: i32,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO agent_tasks (id, plan_id, title, description, status, dependencies, sort_order, created_at, updated_at)
             VALUES (?, ?, ?, '', ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(plan_id)
        .bind(title)
        .bind(status)
        .bind(deps_json)
        .bind(sort_order)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_task failed");
        id.to_string()
    }

    pub async fn insert_task_auto(
        pool: &SqlitePool,
        plan_id: &str,
        title: &str,
        sort_order: i32,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_task(pool, &id, plan_id, title, "pending", "[]", sort_order).await
    }

    /// Insert a workspace artifact, returns the id.
    pub async fn insert_artifact(
        pool: &SqlitePool,
        id: &str,
        chat_id: &str,
        project_id: Option<&str>,
        name: &str,
        content: &str,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO workspace_artifacts (id, chat_id, project_id, name, content_type, content, created_by, updated_by, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'text', ?, 'test', 'test', ?, ?)",
        )
        .bind(id)
        .bind(chat_id)
        .bind(project_id)
        .bind(name)
        .bind(content)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_artifact failed");
        id.to_string()
    }

    pub async fn insert_artifact_auto(
        pool: &SqlitePool,
        chat_id: &str,
        project_id: Option<&str>,
        name: &str,
        content: &str,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_artifact(pool, &id, chat_id, project_id, name, content).await
    }

    /// Insert agent memory, returns the id.
    pub async fn insert_memory(
        pool: &SqlitePool,
        id: &str,
        content: &str,
        category: &str,
        project_id: Option<&str>,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO agent_memory (id, content, category, is_pinned, created_at, updated_at, project_id)
             VALUES (?, ?, ?, 0, ?, ?, ?)",
        )
        .bind(id)
        .bind(content)
        .bind(category)
        .bind(now)
        .bind(now)
        .bind(project_id)
        .execute(pool)
        .await
        .expect("insert_memory failed");
        id.to_string()
    }

    pub async fn insert_memory_auto(
        pool: &SqlitePool,
        content: &str,
        category: &str,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_memory(pool, &id, content, category, None).await
    }

    /// Insert an agent run, returns the id.
    pub async fn insert_agent_run(pool: &SqlitePool, id: &str, chat_id: &str) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO agent_runs (id, chat_id, status, iterations, max_iterations, started_at)
             VALUES (?, ?, 'running', 0, 25, ?)",
        )
        .bind(id)
        .bind(chat_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_agent_run failed");
        id.to_string()
    }

    pub async fn insert_agent_run_auto(pool: &SqlitePool, chat_id: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_agent_run(pool, &id, chat_id).await
    }

    /// Insert a comparison, returns the id.
    pub async fn insert_comparison(pool: &SqlitePool, id: &str) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO comparisons (id, title, left_provider_id, right_provider_id, left_model, right_model, created_at, updated_at)
             VALUES (?, 'Test comparison', 'openrouter', 'openrouter', 'model-a', 'model-b', ?, ?)",
        )
        .bind(id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_comparison failed");
        id.to_string()
    }

    /// Insert a comparison message, returns the id.
    pub async fn insert_comparison_message(
        pool: &SqlitePool,
        id: &str,
        comparison_id: &str,
        role: &str,
        side: Option<&str>,
        content: &str,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO comparison_messages (id, comparison_id, role, side, content, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(comparison_id)
        .bind(role)
        .bind(side)
        .bind(content)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_comparison_message failed");
        id.to_string()
    }

    /// Insert a knowledge base, returns the id.
    pub async fn insert_kb(pool: &SqlitePool, id: &str, name: &str) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO knowledge_bases (id, name, description, embedding_model, created_at, updated_at)
             VALUES (?, ?, '', 'e5-small', ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_kb failed");
        id.to_string()
    }

    pub async fn insert_kb_auto(pool: &SqlitePool, name: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        insert_kb(pool, &id, name).await
    }

    /// Insert a KB document, returns the id.
    pub async fn insert_kb_document(
        pool: &SqlitePool,
        id: &str,
        kb_id: &str,
        name: &str,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO kb_documents (id, kb_id, name, source_type, created_at, updated_at)
             VALUES (?, ?, ?, 'file', ?, ?)",
        )
        .bind(id)
        .bind(kb_id)
        .bind(name)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_kb_document failed");
        id.to_string()
    }

    /// Insert a KB chunk, returns the id.
    pub async fn insert_kb_chunk(
        pool: &SqlitePool,
        id: &str,
        kb_id: &str,
        document_id: &str,
        content: &str,
        chunk_index: i32,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO kb_chunks (id, kb_id, document_id, content, chunk_index, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(kb_id)
        .bind(document_id)
        .bind(content)
        .bind(chunk_index)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_kb_chunk failed");
        id.to_string()
    }

    /// Insert a scheduled task, returns the id.
    pub async fn insert_scheduled_task(
        pool: &SqlitePool,
        id: &str,
        name: &str,
        prompt: &str,
    ) -> String {
        let now = now_secs();
        sqlx::query(
            "INSERT INTO scheduled_tasks (id, name, prompt, cron_expression, enabled, model, run_count, created_at, updated_at)
             VALUES (?, ?, ?, '0 * * * *', 1, 'test-model', 0, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(prompt)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_scheduled_task failed");
        id.to_string()
    }

    /// Insert a notebook with associated KB, returns (notebook_id, kb_id).
    pub async fn insert_notebook_auto(pool: &SqlitePool, name: &str) -> (String, String) {
        let now = now_secs();
        let notebook_id = uuid::Uuid::new_v4().to_string();
        let kb_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO knowledge_bases (id, name, description, embedding_model, notebook_id, created_at, updated_at)
             VALUES (?, ?, '', 'e5-small', ?, ?, ?)",
        )
        .bind(&kb_id)
        .bind(format!("__notebook__{}", name))
        .bind(&notebook_id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_notebook kb failed");

        sqlx::query(
            "INSERT INTO notebooks (id, name, description, kb_id, created_at, updated_at)
             VALUES (?, ?, '', ?, ?, ?)",
        )
        .bind(&notebook_id)
        .bind(name)
        .bind(&kb_id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert_notebook failed");

        (notebook_id, kb_id)
    }

    /// Count rows in a table.
    pub async fn count_rows(pool: &SqlitePool, table: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {}", table);
        let row: (i64,) = sqlx::query_as(&sql)
            .fetch_one(pool)
            .await
            .expect("count_rows failed");
        row.0
    }

    /// Count rows with a WHERE clause.
    pub async fn count_where(pool: &SqlitePool, table: &str, condition: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE {}", table, condition);
        let row: (i64,) = sqlx::query_as(&sql)
            .fetch_one(pool)
            .await
            .expect("count_where failed");
        row.0
    }
}

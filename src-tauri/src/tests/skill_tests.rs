#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_skill() {
        let pool = setup_test_db().await;
        let id = insert_skill_auto(&pool, "Code Writer").await;

        let row: (String, String, i32, i32) = sqlx::query_as(
            "SELECT id, name, is_builtin, enabled FROM skills WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, id);
        assert_eq!(row.1, "Code Writer");
        assert_eq!(row.2, 0); // not builtin
        assert_eq!(row.3, 1); // enabled
    }

    #[tokio::test]
    async fn test_list_skills_ordered() {
        let pool = setup_test_db().await;

        // Insert with explicit sort_order
        let now = chrono::Utc::now().timestamp();
        for (name, order) in [("Beta", 1), ("Alpha", 0), ("Gamma", 2)] {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO skills (id, name, description, icon, content, trigger_description, required_tools, is_builtin, enabled, sort_order, created_at, updated_at)
                 VALUES (?, ?, '', 'code', '', '', '[]', 0, 1, ?, ?, ?)",
            )
            .bind(&id).bind(name).bind(order).bind(now).bind(now)
            .execute(&pool).await.unwrap();
        }

        let rows: Vec<(String, i32)> = sqlx::query_as(
            "SELECT name, sort_order FROM skills ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows[0].0, "Alpha");
        assert_eq!(rows[1].0, "Beta");
        assert_eq!(rows[2].0, "Gamma");
    }

    #[tokio::test]
    async fn test_update_skill() {
        let pool = setup_test_db().await;
        let id = insert_skill_auto(&pool, "Original").await;

        sqlx::query("UPDATE skills SET name = ?, description = ?, updated_at = ? WHERE id = ?")
            .bind("Updated Name")
            .bind("A helpful skill")
            .bind(chrono::Utc::now().timestamp())
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (String, String) = sqlx::query_as(
            "SELECT name, description FROM skills WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Updated Name");
        assert_eq!(row.1, "A helpful skill");
    }

    #[tokio::test]
    async fn test_delete_skill() {
        let pool = setup_test_db().await;
        let id = insert_skill_auto(&pool, "Temp Skill").await;

        assert_eq!(count_rows(&pool, "skills").await, 1);

        sqlx::query("DELETE FROM skills WHERE id = ?")
            .bind(&id).execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "skills").await, 0);
    }

    #[tokio::test]
    async fn test_attach_skill_to_chat() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let skill_id = insert_skill_auto(&pool, "Coder").await;

        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, 'manual', ?)")
            .bind(&chat_id).bind(&skill_id).bind(now)
            .execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "chat_skills").await, 1);

        // Verify join query
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT s.id, s.name FROM skills s INNER JOIN chat_skills cs ON cs.skill_id = s.id WHERE cs.chat_id = ?",
        )
        .bind(&chat_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "Coder");
    }

    #[tokio::test]
    async fn test_attach_duplicate_ignored() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let skill_id = insert_skill_auto(&pool, "Coder").await;

        let now = chrono::Utc::now().timestamp();
        // First attach
        sqlx::query("INSERT OR IGNORE INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, 'manual', ?)")
            .bind(&chat_id).bind(&skill_id).bind(now)
            .execute(&pool).await.unwrap();

        // Duplicate — should be silently ignored
        sqlx::query("INSERT OR IGNORE INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, 'auto', ?)")
            .bind(&chat_id).bind(&skill_id).bind(now)
            .execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "chat_skills").await, 1);
    }

    #[tokio::test]
    async fn test_detach_skill() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let skill_id = insert_skill_auto(&pool, "Coder").await;

        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, 'manual', ?)")
            .bind(&chat_id).bind(&skill_id).bind(now)
            .execute(&pool).await.unwrap();

        sqlx::query("DELETE FROM chat_skills WHERE chat_id = ? AND skill_id = ?")
            .bind(&chat_id).bind(&skill_id)
            .execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "chat_skills").await, 0);
    }

    #[tokio::test]
    async fn test_get_skills_for_chat() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let skill1 = insert_skill_auto(&pool, "Coder").await;
        let skill2 = insert_skill_auto(&pool, "Writer").await;

        let now = chrono::Utc::now().timestamp();
        for skill_id in [&skill1, &skill2] {
            sqlx::query("INSERT INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, 'manual', ?)")
                .bind(&chat_id).bind(skill_id).bind(now)
                .execute(&pool).await.unwrap();
        }

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT s.name FROM skills s INNER JOIN chat_skills cs ON cs.skill_id = s.id WHERE cs.chat_id = ? ORDER BY s.sort_order ASC",
        )
        .bind(&chat_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
    }
}

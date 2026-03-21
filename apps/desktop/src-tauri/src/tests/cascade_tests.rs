#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_delete_chat_cascades_messages() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;
        insert_message_auto(&pool, &chat_id, "user", Some("Hello")).await;
        insert_message_auto(&pool, &chat_id, "assistant", Some("Hi")).await;

        assert_eq!(count_rows(&pool, "messages").await, 2);

        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "messages").await, 0);
    }

    #[tokio::test]
    async fn test_delete_chat_cascades_agent_runs() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        insert_agent_run_auto(&pool, &chat_id).await;
        insert_agent_run_auto(&pool, &chat_id).await;

        assert_eq!(count_rows(&pool, "agent_runs").await, 2);

        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "agent_runs").await, 0);
    }

    #[tokio::test]
    async fn test_delete_chat_cascades_plans() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        insert_plan_auto(&pool, &chat_id, "Build feature").await;

        assert_eq!(count_rows(&pool, "agent_plans").await, 1);

        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "agent_plans").await, 0);
    }

    #[tokio::test]
    async fn test_delete_chat_cascades_chat_skills() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let skill_id = insert_skill_auto(&pool, "Coder").await;

        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, 'manual', ?)")
            .bind(&chat_id).bind(&skill_id).bind(now)
            .execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "chat_skills").await, 1);

        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "chat_skills").await, 0);
    }

    #[tokio::test]
    async fn test_delete_plan_cascades_tasks() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let plan_id = insert_plan_auto(&pool, &chat_id, "Build feature").await;
        insert_task_auto(&pool, &plan_id, "Task 1", 0).await;
        insert_task_auto(&pool, &plan_id, "Task 2", 1).await;
        insert_task_auto(&pool, &plan_id, "Task 3", 2).await;

        assert_eq!(count_rows(&pool, "agent_tasks").await, 3);

        sqlx::query("DELETE FROM agent_plans WHERE id = ?")
            .bind(&plan_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "agent_tasks").await, 0);
    }

    #[tokio::test]
    async fn test_delete_comparison_cascades_messages() {
        let pool = setup_test_db().await;
        let comp_id = uni_common::generate_id();
        insert_comparison(&pool, &comp_id).await;

        let msg_id = uni_common::generate_id();
        insert_comparison_message(&pool, &msg_id, &comp_id, "user", None, "Compare this").await;

        assert_eq!(count_rows(&pool, "comparison_messages").await, 1);

        sqlx::query("DELETE FROM comparisons WHERE id = ?")
            .bind(&comp_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "comparison_messages").await, 0);
    }

    #[tokio::test]
    async fn test_delete_skill_cascades_chat_skills() {
        let pool = setup_test_db().await;
        let chat_id = insert_chat_auto(&pool, "Chat", "assistant").await;
        let skill_id = insert_skill_auto(&pool, "Writer").await;

        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, 'auto', ?)")
            .bind(&chat_id).bind(&skill_id).bind(now)
            .execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "chat_skills").await, 1);

        sqlx::query("DELETE FROM skills WHERE id = ?")
            .bind(&skill_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "chat_skills").await, 0);
    }

    #[tokio::test]
    async fn test_delete_kb_cascades_documents() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "Test KB").await;
        let doc_id = uni_common::generate_id();
        insert_kb_document(&pool, &doc_id, &kb_id, "doc.md").await;

        assert_eq!(count_rows(&pool, "kb_documents").await, 1);

        sqlx::query("DELETE FROM knowledge_bases WHERE id = ?")
            .bind(&kb_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "kb_documents").await, 0);
    }

    #[tokio::test]
    async fn test_delete_kb_cascades_chunks() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "Test KB").await;
        let doc_id = uni_common::generate_id();
        insert_kb_document(&pool, &doc_id, &kb_id, "doc.md").await;

        let chunk_id = uni_common::generate_id();
        insert_kb_chunk(&pool, &chunk_id, &kb_id, &doc_id, "chunk text", 0).await;

        assert_eq!(count_rows(&pool, "kb_chunks").await, 1);

        sqlx::query("DELETE FROM knowledge_bases WHERE id = ?")
            .bind(&kb_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "kb_chunks").await, 0);
    }

    #[tokio::test]
    async fn test_delete_kb_document_cascades_chunks() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "Test KB").await;
        let doc_id = uni_common::generate_id();
        insert_kb_document(&pool, &doc_id, &kb_id, "doc.md").await;

        let chunk1 = uni_common::generate_id();
        let chunk2 = uni_common::generate_id();
        insert_kb_chunk(&pool, &chunk1, &kb_id, &doc_id, "chunk 1", 0).await;
        insert_kb_chunk(&pool, &chunk2, &kb_id, &doc_id, "chunk 2", 1).await;

        assert_eq!(count_rows(&pool, "kb_chunks").await, 2);

        sqlx::query("DELETE FROM kb_documents WHERE id = ?")
            .bind(&doc_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "kb_chunks").await, 0);
        // KB itself still exists
        assert_eq!(count_rows(&pool, "knowledge_bases").await, 1);
    }

    #[tokio::test]
    async fn test_delete_folder_nullifies_chat() {
        let pool = setup_test_db().await;
        let folder_id = insert_folder_auto(&pool, "Folder", "chat").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;

        sqlx::query("UPDATE chats SET folder_id = ? WHERE id = ?")
            .bind(&folder_id)
            .bind(&chat_id)
            .execute(&pool)
            .await
            .unwrap();

        // Verify assignment
        let row: (Option<String>,) = sqlx::query_as("SELECT folder_id FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0.as_deref(), Some(folder_id.as_str()));

        // Delete folder — ON DELETE SET NULL
        sqlx::query("DELETE FROM folders WHERE id = ?")
            .bind(&folder_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (Option<String>,) = sqlx::query_as("SELECT folder_id FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(row.0.is_none(), "folder_id should be NULL after folder deletion");

        // Chat still exists
        assert_eq!(count_rows(&pool, "chats").await, 1);
    }
}

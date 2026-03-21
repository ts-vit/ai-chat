#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;
    use crate::services::kb_fts;

    #[tokio::test]
    async fn test_create_knowledge_base() {
        let pool = setup_test_db().await;
        let id = insert_kb_auto(&pool, "My KB").await;

        let row: (String, String, String) = sqlx::query_as(
            "SELECT id, name, embedding_model FROM knowledge_bases WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, id);
        assert_eq!(row.1, "My KB");
        assert_eq!(row.2, "e5-small");
    }

    #[tokio::test]
    async fn test_list_knowledge_bases() {
        let pool = setup_test_db().await;
        let now = chrono::Utc::now().timestamp();

        for (i, name) in ["KB A", "KB B", "KB C"].iter().enumerate() {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO knowledge_bases (id, name, description, embedding_model, created_at, updated_at) VALUES (?, ?, '', 'e5-small', ?, ?)",
            )
            .bind(&id).bind(name).bind(now + i as i64).bind(now + i as i64)
            .execute(&pool).await.unwrap();
        }

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM knowledge_bases ORDER BY created_at DESC",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "KB C"); // most recent first
    }

    #[tokio::test]
    async fn test_create_document() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "KB").await;
        let doc_id = uuid::Uuid::new_v4().to_string();
        insert_kb_document(&pool, &doc_id, &kb_id, "readme.md").await;

        let row: (String, String, String) = sqlx::query_as(
            "SELECT id, name, source_type FROM kb_documents WHERE id = ?",
        )
        .bind(&doc_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, doc_id);
        assert_eq!(row.1, "readme.md");
        assert_eq!(row.2, "file");
    }

    #[tokio::test]
    async fn test_create_chunks() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "KB").await;
        let doc_id = uuid::Uuid::new_v4().to_string();
        insert_kb_document(&pool, &doc_id, &kb_id, "doc.md").await;

        for i in 0..3 {
            let chunk_id = uuid::Uuid::new_v4().to_string();
            insert_kb_chunk(&pool, &chunk_id, &kb_id, &doc_id, &format!("chunk {}", i), i).await;
        }

        let chunk_count = count_where(
            &pool,
            "kb_chunks",
            &format!("document_id = '{}'", doc_id),
        )
        .await;

        assert_eq!(chunk_count, 3);
    }

    #[tokio::test]
    async fn test_update_kb() {
        let pool = setup_test_db().await;
        let id = insert_kb_auto(&pool, "Old Name").await;

        sqlx::query("UPDATE knowledge_bases SET name = ?, description = ?, updated_at = ? WHERE id = ?")
            .bind("New Name")
            .bind("A knowledge base for testing")
            .bind(chrono::Utc::now().timestamp())
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (String, String) = sqlx::query_as(
            "SELECT name, description FROM knowledge_bases WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "New Name");
        assert_eq!(row.1, "A knowledge base for testing");
    }

    #[tokio::test]
    async fn test_delete_kb_cascades() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "KB").await;
        let doc_id = uuid::Uuid::new_v4().to_string();
        insert_kb_document(&pool, &doc_id, &kb_id, "doc.md").await;
        let chunk_id = uuid::Uuid::new_v4().to_string();
        insert_kb_chunk(&pool, &chunk_id, &kb_id, &doc_id, "chunk", 0).await;

        assert_eq!(count_rows(&pool, "kb_documents").await, 1);
        assert_eq!(count_rows(&pool, "kb_chunks").await, 1);

        sqlx::query("DELETE FROM knowledge_bases WHERE id = ?")
            .bind(&kb_id).execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "kb_documents").await, 0);
        assert_eq!(count_rows(&pool, "kb_chunks").await, 0);
    }

    #[tokio::test]
    async fn test_delete_document_cascades_chunks() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "KB").await;
        let doc_id = uuid::Uuid::new_v4().to_string();
        insert_kb_document(&pool, &doc_id, &kb_id, "doc.md").await;

        let c1 = uuid::Uuid::new_v4().to_string();
        let c2 = uuid::Uuid::new_v4().to_string();
        insert_kb_chunk(&pool, &c1, &kb_id, &doc_id, "chunk 1", 0).await;
        insert_kb_chunk(&pool, &c2, &kb_id, &doc_id, "chunk 2", 1).await;

        sqlx::query("DELETE FROM kb_documents WHERE id = ?")
            .bind(&doc_id).execute(&pool).await.unwrap();

        assert_eq!(count_rows(&pool, "kb_chunks").await, 0);
        assert_eq!(count_rows(&pool, "knowledge_bases").await, 1); // KB still there
    }

    #[tokio::test]
    async fn test_attach_kb_to_chat() {
        let pool = setup_test_db().await;
        let kb_id = insert_kb_auto(&pool, "KB").await;
        let chat_id = insert_chat_auto(&pool, "Chat", "chat").await;

        sqlx::query("UPDATE chats SET kb_id = ? WHERE id = ?")
            .bind(&kb_id).bind(&chat_id)
            .execute(&pool).await.unwrap();

        let row: (Option<String>,) = sqlx::query_as("SELECT kb_id FROM chats WHERE id = ?")
            .bind(&chat_id).fetch_one(&pool).await.unwrap();

        assert_eq!(row.0, Some(kb_id));
    }

    #[tokio::test]
    async fn test_search_all_kb_fts() {
        let pool = setup_test_db().await;

        // Create 2 KBs
        let kb1 = insert_kb_auto(&pool, "KB Alpha").await;
        let kb2 = insert_kb_auto(&pool, "KB Beta").await;

        // Create documents
        let doc1 = uuid::Uuid::new_v4().to_string();
        let doc2 = uuid::Uuid::new_v4().to_string();
        insert_kb_document(&pool, &doc1, &kb1, "rust_guide.md").await;
        insert_kb_document(&pool, &doc2, &kb2, "python_guide.md").await;

        // Create chunks
        let c1 = uuid::Uuid::new_v4().to_string();
        let c2 = uuid::Uuid::new_v4().to_string();
        let c3 = uuid::Uuid::new_v4().to_string();
        insert_kb_chunk(&pool, &c1, &kb1, &doc1, "Rust is a systems programming language", 0).await;
        insert_kb_chunk(&pool, &c2, &kb1, &doc1, "Memory safety without garbage collection", 1).await;
        insert_kb_chunk(&pool, &c3, &kb2, &doc2, "Python is a dynamic programming language", 0).await;

        // Index into FTS
        kb_fts::index_kb_chunks(&pool, &[
            (c1.clone(), kb1.clone(), doc1.clone(), "Rust is a systems programming language".to_string()),
            (c2.clone(), kb1.clone(), doc1.clone(), "Memory safety without garbage collection".to_string()),
            (c3.clone(), kb2.clone(), doc2.clone(), "Python is a dynamic programming language".to_string()),
        ]).await.unwrap();

        // Search for "programming" — should find chunks from both KBs
        let results = kb_fts::search_all_kb_fts(&pool, "programming", 10).await.unwrap();
        assert_eq!(results.len(), 2);

        // Search for "Rust" — should find only KB1 chunks
        let results = kb_fts::search_all_kb_fts(&pool, "Rust", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk_id, c1);

        // Search for "memory" — should find only KB1 chunk 2
        let results = kb_fts::search_all_kb_fts(&pool, "memory", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk_id, c2);

        // Empty query
        let results = kb_fts::search_all_kb_fts(&pool, "", 10).await.unwrap();
        assert_eq!(results.len(), 0);

        // Search with limit
        let results = kb_fts::search_all_kb_fts(&pool, "programming", 1).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}

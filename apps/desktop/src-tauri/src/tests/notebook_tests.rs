#[cfg(test)]
mod tests {
    use crate::test_db::test_helpers::*;

    #[tokio::test]
    async fn test_create_notebook_with_kb() {
        let pool = setup_test_db().await;
        let (notebook_id, kb_id) = insert_notebook_auto(&pool, "My Notebook").await;

        // Notebook exists
        let row: (String, String, Option<String>) = sqlx::query_as(
            "SELECT id, name, kb_id FROM notebooks WHERE id = ?",
        )
        .bind(&notebook_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, notebook_id);
        assert_eq!(row.1, "My Notebook");
        assert_eq!(row.2, Some(kb_id.clone()));

        // KB exists with notebook_id set
        let kb_row: (String, Option<String>) = sqlx::query_as(
            "SELECT id, notebook_id FROM knowledge_bases WHERE id = ?",
        )
        .bind(&kb_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(kb_row.0, kb_id);
        assert_eq!(kb_row.1, Some(notebook_id));
    }

    #[tokio::test]
    async fn test_notebook_kb_hidden_from_list() {
        let pool = setup_test_db().await;

        // Create a regular KB
        let _regular_kb = insert_kb_auto(&pool, "Regular KB").await;

        // Create a notebook (which creates a KB with notebook_id)
        let (_notebook_id, _notebook_kb_id) = insert_notebook_auto(&pool, "My Notebook").await;

        // Total KBs should be 2
        assert_eq!(count_rows(&pool, "knowledge_bases").await, 2);

        // But filtering with notebook_id IS NULL should return only 1
        let visible: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM knowledge_bases WHERE notebook_id IS NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(visible, 1);
    }

    #[tokio::test]
    async fn test_delete_notebook_cascades() {
        let pool = setup_test_db().await;
        let (notebook_id, kb_id) = insert_notebook_auto(&pool, "To Delete").await;

        // Add a document to the KB
        let doc_id = uni_common::generate_id();
        insert_kb_document(&pool, &doc_id, &kb_id, "test.md").await;

        // Add a chunk
        let chunk_id = uni_common::generate_id();
        insert_kb_chunk(&pool, &chunk_id, &kb_id, &doc_id, "some content", 0).await;

        assert_eq!(count_rows(&pool, "notebooks").await, 1);
        assert_eq!(count_rows(&pool, "knowledge_bases").await, 1);
        assert_eq!(count_rows(&pool, "kb_documents").await, 1);
        assert_eq!(count_rows(&pool, "kb_chunks").await, 1);

        // Delete KB first (simulates cascade in delete_notebook command)
        sqlx::query("DELETE FROM knowledge_bases WHERE id = ?")
            .bind(&kb_id)
            .execute(&pool)
            .await
            .unwrap();

        // Then delete notebook
        sqlx::query("DELETE FROM notebooks WHERE id = ?")
            .bind(&notebook_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count_rows(&pool, "notebooks").await, 0);
        assert_eq!(count_rows(&pool, "knowledge_bases").await, 0);
        assert_eq!(count_rows(&pool, "kb_documents").await, 0);
        assert_eq!(count_rows(&pool, "kb_chunks").await, 0);
    }

    #[tokio::test]
    async fn test_list_notebooks_with_stats() {
        let pool = setup_test_db().await;
        let (notebook_id, kb_id) = insert_notebook_auto(&pool, "Stats Notebook").await;

        // Update KB counters
        sqlx::query(
            "UPDATE knowledge_bases SET document_count = 3, total_chunks = 42 WHERE id = ?",
        )
        .bind(&kb_id)
        .execute(&pool)
        .await
        .unwrap();

        // Query with JOIN (same as list_notebooks command)
        let row: (String, String, i64, i64) = sqlx::query_as(
            "SELECT n.id, n.name, COALESCE(kb.document_count, 0), COALESCE(kb.total_chunks, 0) \
             FROM notebooks n LEFT JOIN knowledge_bases kb ON n.kb_id = kb.id \
             WHERE n.id = ?",
        )
        .bind(&notebook_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.1, "Stats Notebook");
        assert_eq!(row.2, 3); // document_count
        assert_eq!(row.3, 42); // total_chunks
    }

    #[tokio::test]
    async fn test_update_notebook() {
        let pool = setup_test_db().await;
        let (notebook_id, _kb_id) = insert_notebook_auto(&pool, "Old Name").await;

        sqlx::query("UPDATE notebooks SET name = ?, description = ? WHERE id = ?")
            .bind("New Name")
            .bind("Updated description")
            .bind(&notebook_id)
            .execute(&pool)
            .await
            .unwrap();

        let row: (String, String) = sqlx::query_as(
            "SELECT name, description FROM notebooks WHERE id = ?",
        )
        .bind(&notebook_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "New Name");
        assert_eq!(row.1, "Updated description");
    }

    #[tokio::test]
    async fn test_notebook_documents_via_kb() {
        let pool = setup_test_db().await;
        let (_notebook_id, kb_id) = insert_notebook_auto(&pool, "Doc Notebook").await;

        // Add documents through the underlying KB
        let doc1 = uni_common::generate_id();
        let doc2 = uni_common::generate_id();
        insert_kb_document(&pool, &doc1, &kb_id, "file1.md").await;
        insert_kb_document(&pool, &doc2, &kb_id, "file2.pdf").await;

        // List documents for the KB (same as list_notebook_documents)
        let count = count_where(
            &pool,
            "kb_documents",
            &format!("kb_id = '{}'", kb_id),
        )
        .await;

        assert_eq!(count, 2);
    }
}

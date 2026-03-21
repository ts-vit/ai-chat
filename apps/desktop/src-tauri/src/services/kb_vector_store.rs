// KB Vector Store: LanceDB wrapper for knowledge base chunk embeddings

use anyhow::Result;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int32Array, RecordBatch, RecordBatchReader,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

const DEFAULT_EMBEDDING_DIM: i32 = 1536;

fn vector_fixed_dim(schema: &Schema) -> Option<i32> {
    let f = schema.field_with_name("vector").ok()?;
    match f.data_type() {
        DataType::FixedSizeList(_, size) => Some(*size),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct KbVectorEntry {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct KbSearchResult {
    pub chunk_id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub score: f32,
}

fn kb_schema(dim: i32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("document_id", DataType::Utf8, false),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dim,
            ),
            false,
        ),
    ]))
}

fn entries_to_batch(entries: &[KbVectorEntry], schema: &Schema, dim: i32) -> Result<RecordBatch> {
    let n = entries.len();
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    let doc_ids: Vec<&str> = entries.iter().map(|e| e.document_id.as_str()).collect();
    let indices: Vec<i32> = entries.iter().map(|e| e.chunk_index).collect();

    let mut all_vectors: Vec<f32> = Vec::with_capacity(n * dim as usize);
    for entry in entries {
        if entry.vector.len() != dim as usize {
            anyhow::bail!(
                "vector length {} != {}",
                entry.vector.len(),
                dim
            );
        }
        all_vectors.extend(&entry.vector);
    }

    let vector_values = Arc::new(Float32Array::from(all_vectors));
    let vector_array = Arc::new(
        FixedSizeListArray::try_new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            dim,
            vector_values,
            None,
        )
        .map_err(|e| anyhow::anyhow!("FixedSizeListArray: {}", e))?,
    );

    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(doc_ids)),
            Arc::new(Int32Array::from(indices)),
            vector_array,
        ],
    )?;
    Ok(batch)
}

/// Sanitize KB ID for use as LanceDB table name
fn table_name(kb_id: &str) -> String {
    format!("kb_{}", kb_id.replace('-', "_"))
}

pub struct KbVectorStore {
    db_path: String,
}

impl KbVectorStore {
    pub async fn new(db_path: &str) -> Result<Self> {
        let _ = std::fs::create_dir_all(db_path);
        Ok(Self {
            db_path: db_path.to_string(),
        })
    }

    async fn ensure_vector_dimension(&self, kb_id: &str, dim: i32) -> Result<()> {
        let conn = connect(&self.db_path).execute().await?;
        let tbl = table_name(kb_id);
        let names = conn.table_names().limit(1000).execute().await?;
        if !names.iter().any(|n| n == &tbl) {
            return Ok(());
        }
        let table = conn.open_table(&tbl).execute().await?;
        let schema_ref = table.schema().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        if let Some(old) = vector_fixed_dim(schema_ref.as_ref()) {
            if old != dim {
                log::warn!(
                    "KB embedding dimensions changed ({} → {}) for {}, resetting vector index. FTS remains available. Reindex KB to rebuild semantic search.",
                    old,
                    dim,
                    kb_id
                );
                conn.drop_table(&tbl, &[]).await?;
            }
        }
        Ok(())
    }

    pub async fn ensure_table(&self, kb_id: &str, dimensions: Option<i32>) -> Result<()> {
        let dim = dimensions.unwrap_or(DEFAULT_EMBEDDING_DIM);
        self.ensure_vector_dimension(kb_id, dim).await?;
        let conn = connect(&self.db_path).execute().await?;
        let tbl = table_name(kb_id);
        let names = conn.table_names().limit(1000).execute().await?;
        if !names.iter().any(|n| n == &tbl) {
            let schema = kb_schema(dim);
            // Create empty table with schema
            let batch = RecordBatch::new_empty(schema.clone());
            let reader: Box<dyn RecordBatchReader + Send> =
                Box::new(arrow_array::RecordBatchIterator::new(
                    vec![Ok(batch)].into_iter(),
                    schema,
                ));
            conn.create_table(&tbl, reader).execute().await?;
        }
        Ok(())
    }

    pub async fn add_chunks(&self, kb_id: &str, chunks: Vec<KbVectorEntry>, dimensions: Option<i32>) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }
        let dim = dimensions.unwrap_or(DEFAULT_EMBEDDING_DIM);
        self.ensure_vector_dimension(kb_id, dim).await?;
        let conn = connect(&self.db_path).execute().await?;
        let tbl = table_name(kb_id);
        let schema = kb_schema(dim);
        let batch = entries_to_batch(&chunks, &schema, dim)?;

        let names = conn.table_names().limit(1000).execute().await?;
        if names.iter().any(|n| n == &tbl) {
            let table = conn.open_table(&tbl).execute().await?;
            let reader: Box<dyn RecordBatchReader + Send> =
                Box::new(arrow_array::RecordBatchIterator::new(
                    vec![Ok(batch)].into_iter(),
                    schema,
                ));
            table.add(reader).execute().await?;
        } else {
            let reader: Box<dyn RecordBatchReader + Send> =
                Box::new(arrow_array::RecordBatchIterator::new(
                    vec![Ok(batch)].into_iter(),
                    schema,
                ));
            conn.create_table(&tbl, reader).execute().await?;
        }
        Ok(())
    }

    pub async fn search(
        &self,
        kb_id: &str,
        query_embedding: &[f32],
        top_k: usize,
        min_score: f32,
        dimensions: Option<i32>,
    ) -> Result<Vec<KbSearchResult>> {
        let dim = dimensions.unwrap_or(DEFAULT_EMBEDDING_DIM);
        self.ensure_vector_dimension(kb_id, dim).await?;
        let conn = connect(&self.db_path).execute().await?;
        let tbl = table_name(kb_id);
        let names = conn.table_names().limit(1000).execute().await?;
        if !names.iter().any(|n| n == &tbl) {
            return Ok(Vec::new());
        }

        let table = conn.open_table(&tbl).execute().await?;
        let q = table.query().nearest_to(query_embedding)?.limit(top_k);
        let batches: Vec<RecordBatch> = q.execute().await?.try_collect().await?;

        let mut results = Vec::new();
        for batch in batches {
            let schema = batch.schema();
            let id_idx = schema.column_with_name("id").map(|(i, _)| i);
            let doc_idx = schema.column_with_name("document_id").map(|(i, _)| i);
            let idx_idx = schema.column_with_name("chunk_index").map(|(i, _)| i);
            let score_idx = schema
                .column_with_name("_distance")
                .or_else(|| schema.column_with_name("_score"))
                .map(|(i, _)| i);

            for row in 0..batch.num_rows() {
                let score = score_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<Float32Array>())
                    .map(|a| 1.0 - a.value(row))
                    .unwrap_or(0.0);

                if score < min_score {
                    continue;
                }

                let chunk_id = id_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string())
                    .unwrap_or_default();
                let document_id = doc_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string())
                    .unwrap_or_default();
                let chunk_index = idx_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<Int32Array>())
                    .map(|a| a.value(row))
                    .unwrap_or(0);

                results.push(KbSearchResult {
                    chunk_id,
                    document_id,
                    chunk_index,
                    score,
                });
            }
        }
        Ok(results)
    }

    pub async fn delete_document_chunks(&self, kb_id: &str, document_id: &str) -> Result<()> {
        let conn = connect(&self.db_path).execute().await?;
        let tbl = table_name(kb_id);
        let names = conn.table_names().limit(1000).execute().await?;
        if !names.iter().any(|n| n == &tbl) {
            return Ok(());
        }
        let table = conn.open_table(&tbl).execute().await?;
        let pred = format!("document_id = '{}'", document_id.replace('\'', "''"));
        table.delete(&pred).await?;
        Ok(())
    }

    pub async fn delete_kb(&self, kb_id: &str) -> Result<()> {
        let conn = connect(&self.db_path).execute().await?;
        let tbl = table_name(kb_id);
        let names = conn.table_names().limit(1000).execute().await?;
        if names.iter().any(|n| n == &tbl) {
            conn.drop_table(&tbl, &[]).await?;
        }
        Ok(())
    }
}

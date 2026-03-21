// Memory vector store: LanceDB table for agent memory embeddings

use anyhow::Result;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchReader, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

const TABLE_NAME: &str = "agent_memory_vectors";

fn fixed_list_dim(schema: &Schema, column: &str) -> Option<i32> {
    let f = schema.field_with_name(column).ok()?;
    match f.data_type() {
        DataType::FixedSizeList(_, size) => Some(*size),
        _ => None,
    }
}

fn make_schema(embedding_dim: i32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                embedding_dim,
            ),
            false,
        ),
    ]))
}

fn record_to_batch(
    id: &str,
    embedding: &[f32],
    schema: &Schema,
    embedding_dim: i32,
) -> Result<RecordBatch> {
    if embedding.len() != embedding_dim as usize {
        anyhow::bail!(
            "embedding length {} != {}",
            embedding.len(),
            embedding_dim
        );
    }
    let embedding_values = Arc::new(Float32Array::from(embedding.to_vec()));
    let embedding_array = Arc::new(
        FixedSizeListArray::try_new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            embedding_dim,
            embedding_values,
            None,
        )
        .map_err(|e| anyhow::anyhow!("FixedSizeListArray: {}", e))?,
    );

    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(StringArray::from(vec![id])),
            embedding_array,
        ],
    )?;
    Ok(batch)
}

pub struct MemoryVectorStore {
    db_path: String,
    embedding_dim: usize,
}

impl MemoryVectorStore {
    pub async fn new(db_path: &str, embedding_dim: usize) -> Result<Self> {
        let _ = std::fs::create_dir_all(db_path);
        Ok(Self {
            db_path: db_path.to_string(),
            embedding_dim,
        })
    }

    fn dim_i32(&self) -> i32 {
        self.embedding_dim as i32
    }

    async fn ensure_embedding_schema(&self) -> Result<()> {
        let conn = connect(&self.db_path).execute().await?;
        let names = conn.table_names().limit(1000).execute().await?;
        if !names.iter().any(|n| n == TABLE_NAME) {
            return Ok(());
        }
        let table = conn.open_table(TABLE_NAME).execute().await?;
        let schema_ref = table.schema().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        let actual = fixed_list_dim(schema_ref.as_ref(), "embedding");
        let expected = self.embedding_dim;
        if let Some(old) = actual {
            if old as usize != expected {
                log::warn!(
                    "Embedding dimensions changed ({} → {}), resetting vector index. FTS remains available. Run reindex to rebuild semantic search.",
                    old,
                    expected
                );
                conn.drop_table(TABLE_NAME, &[]).await?;
            }
        }
        Ok(())
    }

    async fn has_table(&self) -> Result<bool> {
        let conn = connect(&self.db_path).execute().await?;
        let names = conn.table_names().limit(1000).execute().await?;
        Ok(names.iter().any(|n| n == TABLE_NAME))
    }

    pub async fn save(&self, id: &str, embedding: &[f32]) -> Result<()> {
        self.ensure_embedding_schema().await?;
        let conn = connect(&self.db_path).execute().await?;
        let dim = self.dim_i32();
        let s = make_schema(dim);
        let batch = record_to_batch(id, embedding, &s, dim)?;

        let names = conn.table_names().limit(1000).execute().await?;
        let has_table = names.iter().any(|n| n == TABLE_NAME);

        if has_table {
            let table = conn.open_table(TABLE_NAME).execute().await?;
            let reader: Box<dyn RecordBatchReader + Send> =
                Box::new(arrow_array::RecordBatchIterator::new(
                    vec![Ok(batch)].into_iter(),
                    s.clone(),
                ));
            table.add(reader).execute().await?;
        } else {
            let reader: Box<dyn RecordBatchReader + Send> =
                Box::new(arrow_array::RecordBatchIterator::new(
                    vec![Ok(batch)].into_iter(),
                    s.clone(),
                ));
            conn.create_table(TABLE_NAME, reader).execute().await?;
        }
        Ok(())
    }

    pub async fn update(&self, id: &str, embedding: &[f32]) -> Result<()> {
        self.delete(id).await?;
        self.save(id, embedding).await?;
        Ok(())
    }

    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>> {
        self.ensure_embedding_schema().await?;
        if !self.has_table().await? {
            return Ok(Vec::new());
        }
        let conn = connect(&self.db_path).execute().await?;
        let table = conn.open_table(TABLE_NAME).execute().await?;
        let q = table.query().nearest_to(query_embedding)?.limit(limit);
        let batches: Vec<RecordBatch> = q.execute().await?.try_collect().await?;

        let mut results = Vec::new();
        for batch in batches {
            let batch_schema = batch.schema();
            let id_idx = batch_schema.column_with_name("id").map(|(i, _)| i);
            let score_idx = batch_schema
                .column_with_name("_score")
                .or_else(|| batch_schema.column_with_name("_distance"))
                .map(|(i, _)| i);

            for row in 0..batch.num_rows() {
                let id = id_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string())
                    .unwrap_or_default();
                let distance = score_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<Float32Array>())
                    .map(|a| a.value(row))
                    .unwrap_or(0.0);
                // Convert distance to similarity: 1.0 - distance
                let similarity = (1.0 - distance).max(0.0).min(1.0);
                results.push((id, similarity));
            }
        }
        Ok(results)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        self.ensure_embedding_schema().await?;
        if !self.has_table().await? {
            return Ok(());
        }
        let conn = connect(&self.db_path).execute().await?;
        let table = conn.open_table(TABLE_NAME).execute().await?;
        let pred = format!("id = '{}'", id.replace('\'', "''"));
        table.delete(&pred).await?;
        Ok(())
    }

    pub async fn delete_all(&self) -> Result<()> {
        self.ensure_embedding_schema().await?;
        if !self.has_table().await? {
            return Ok(());
        }
        let conn = connect(&self.db_path).execute().await?;
        let table = conn.open_table(TABLE_NAME).execute().await?;
        table.delete("id IS NOT NULL").await?;
        Ok(())
    }

    pub async fn count(&self) -> Result<usize> {
        self.ensure_embedding_schema().await?;
        if !self.has_table().await? {
            return Ok(0);
        }
        let conn = connect(&self.db_path).execute().await?;
        let table = conn.open_table(TABLE_NAME).execute().await?;
        Ok(table.count_rows(None).await?)
    }
}

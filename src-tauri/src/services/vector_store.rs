// Vector store: LanceDB обёртка для message_chunks (эмбеддинги + метаданные)

use anyhow::Result;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int32Array, Int64Array, RecordBatch, RecordBatchReader,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

const TABLE_NAME: &str = "message_chunks";
const EMBEDDING_DIM: i32 = 384;

#[derive(Debug, Clone)]
pub struct ChunkRecord {
    pub id: String,
    pub message_id: String,
    pub chat_id: String,
    pub chunk_text: String,
    pub embedding: Vec<f32>,
    pub chunk_index: i32,
    pub role: String,
    pub source: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub message_id: String,
    pub chat_id: String,
    pub chunk_text: String,
    pub role: String,
    pub score: f32,
    pub created_at: i64,
}

fn schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("message_id", DataType::Utf8, false),
        Field::new("chat_id", DataType::Utf8, false),
        Field::new("chunk_text", DataType::Utf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            false,
        ),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new("role", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("created_at", DataType::Int64, false),
    ]))
}

fn chunks_to_batch(chunks: &[ChunkRecord], schema: &Schema) -> Result<RecordBatch> {
    let n = chunks.len();
    let ids: Vec<&str> = chunks.iter().map(|c| c.id.as_str()).collect();
    let message_ids: Vec<&str> = chunks.iter().map(|c| c.message_id.as_str()).collect();
    let chat_ids: Vec<&str> = chunks.iter().map(|c| c.chat_id.as_str()).collect();
    let texts: Vec<&str> = chunks.iter().map(|c| c.chunk_text.as_str()).collect();
    let chunk_indices: Vec<i32> = chunks.iter().map(|c| c.chunk_index).collect();
    let roles: Vec<&str> = chunks.iter().map(|c| c.role.as_str()).collect();
    let sources: Vec<&str> = chunks.iter().map(|c| c.source.as_str()).collect();
    let created_ats: Vec<i64> = chunks.iter().map(|c| c.created_at).collect();

    let mut all_embeddings: Vec<f32> = Vec::with_capacity(n * EMBEDDING_DIM as usize);
    for c in chunks {
        if c.embedding.len() != EMBEDDING_DIM as usize {
            anyhow::bail!(
                "embedding length {} != {}",
                c.embedding.len(),
                EMBEDDING_DIM
            );
        }
        all_embeddings.extend(&c.embedding);
    }
    let embedding_values = Arc::new(Float32Array::from(all_embeddings));
    let embedding_array = Arc::new(
        FixedSizeListArray::try_new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            EMBEDDING_DIM,
            embedding_values,
            None,
        )
        .map_err(|e| anyhow::anyhow!("FixedSizeListArray: {}", e))?,
    );

    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(message_ids)),
            Arc::new(StringArray::from(chat_ids)),
            Arc::new(StringArray::from(texts)),
            embedding_array,
            Arc::new(Int32Array::from(chunk_indices)),
            Arc::new(StringArray::from(roles)),
            Arc::new(StringArray::from(sources)),
            Arc::new(Int64Array::from(created_ats)),
        ],
    )?;
    Ok(batch)
}

pub struct VectorStore {
    db_path: String,
}

impl VectorStore {
    /// Подключиться/создать БД в указанной директории.
    pub async fn new(db_path: &str) -> Result<Self> {
        let _ = std::fs::create_dir_all(db_path);
        Ok(Self {
            db_path: db_path.to_string(),
        })
    }

    /// Сохранить чанки с эмбеддингами.
    pub async fn save_chunks(&self, chunks: Vec<ChunkRecord>) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }
        let conn = connect(&self.db_path).execute().await?;
        let schema = schema();
        let batch = chunks_to_batch(&chunks, &schema)?;
        let names = conn.table_names().limit(1000).execute().await?;
        let has_table = names.iter().any(|n| n == TABLE_NAME);

        if has_table {
            let table = conn.open_table(TABLE_NAME).execute().await?;
            let reader: Box<dyn RecordBatchReader + Send> =
                Box::new(arrow_array::RecordBatchIterator::new(
                    vec![Ok(batch)].into_iter(),
                    schema.clone(),
                ));
            table.add(reader).execute().await?;
        } else {
            let reader: Box<dyn RecordBatchReader + Send> =
                Box::new(arrow_array::RecordBatchIterator::new(
                    vec![Ok(batch)].into_iter(),
                    schema.clone(),
                ));
            conn.create_table(TABLE_NAME, reader).execute().await?;
        }
        Ok(())
    }

    /// Поиск по вектору, вернуть top-N ближайших (cosine для нормализованных векторов).
    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        source_filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let conn = connect(&self.db_path).execute().await?;
        let table = conn.open_table(TABLE_NAME).execute().await?;
        let mut q = table.query().nearest_to(query_embedding)?.limit(limit);
        if let Some(filter) = source_filter {
            q = q.only_if(format!("source = '{}'", filter.replace('\'', "''")));
        }
        let batches: Vec<RecordBatch> = q.execute().await?.try_collect().await?;
        let mut results = Vec::new();
        for batch in batches {
            let schema = batch.schema();
            let message_id_idx = schema.column_with_name("message_id").map(|(i, _)| i);
            let chat_id_idx = schema.column_with_name("chat_id").map(|(i, _)| i);
            let text_idx = schema.column_with_name("chunk_text").map(|(i, _)| i);
            let role_idx = schema.column_with_name("role").map(|(i, _)| i);
            let created_idx = schema.column_with_name("created_at").map(|(i, _)| i);
            let score_idx = schema
                .column_with_name("_score")
                .or_else(|| schema.column_with_name("_distance"))
                .map(|(i, _)| i);

            for row in 0..batch.num_rows() {
                let message_id = message_id_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a: &StringArray| a.value(row).to_string())
                    .unwrap_or_default();
                let chat_id = chat_id_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a: &StringArray| a.value(row).to_string())
                    .unwrap_or_default();
                let chunk_text = text_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a: &StringArray| a.value(row).to_string())
                    .unwrap_or_default();
                let role = role_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a: &StringArray| a.value(row).to_string())
                    .unwrap_or_default();
                let created_at = created_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<Int64Array>())
                    .map(|a: &Int64Array| a.value(row))
                    .unwrap_or(0);
                let score = score_idx
                    .and_then(|i| batch.column(i).as_any().downcast_ref::<Float32Array>())
                    .map(|a: &Float32Array| a.value(row))
                    .unwrap_or(0.0);
                results.push(SearchResult {
                    message_id,
                    chat_id,
                    chunk_text,
                    role,
                    score,
                    created_at,
                });
            }
        }
        Ok(results)
    }

    /// Удалить все чанки для сообщения.
    pub async fn delete_by_message_id(&self, message_id: &str) -> Result<()> {
        let conn = connect(&self.db_path).execute().await?;
        let table = conn.open_table(TABLE_NAME).execute().await?;
        let pred = format!("message_id = '{}'", message_id.replace('\'', "''"));
        table.delete(&pred).await?;
        Ok(())
    }

    /// Удалить все чанки для чата.
    pub async fn delete_by_chat_id(&self, chat_id: &str) -> Result<()> {
        let conn = connect(&self.db_path).execute().await?;
        let table = conn.open_table(TABLE_NAME).execute().await?;
        let pred = format!("chat_id = '{}'", chat_id.replace('\'', "''"));
        table.delete(&pred).await?;
        Ok(())
    }

    /// Проверить, есть ли уже эмбеддинги для сообщения.
    pub async fn has_message(&self, message_id: &str) -> Result<bool> {
        let conn = connect(&self.db_path).execute().await?;
        let names = conn.table_names().limit(1000).execute().await?;
        if !names.iter().any(|n| n == TABLE_NAME) {
            return Ok(false);
        }
        let table = conn.open_table(TABLE_NAME).execute().await?;
        let pred = format!("message_id = '{}'", message_id.replace('\'', "''"));
        let n = table.count_rows(Some(pred)).await?;
        Ok(n > 0)
    }

    /// Количество записей в индексе.
    pub async fn count(&self) -> Result<usize> {
        let conn = connect(&self.db_path).execute().await?;
        let names = conn.table_names().limit(1000).execute().await?;
        if !names.iter().any(|n| n == TABLE_NAME) {
            return Ok(0);
        }
        let table = conn.open_table(TABLE_NAME).execute().await?;
        Ok(table.count_rows(None).await.map_err(|e| anyhow::anyhow!("{}", e))?)
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Notebook {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kb_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotebookWithStats {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kb_id: Option<String>,
    pub document_count: i64,
    pub total_chunks: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

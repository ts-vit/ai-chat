use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PromptLibraryItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub category: String,
    pub is_builtin: bool,
    pub language: String,
    pub created_at: i64,
    pub updated_at: i64,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub chat_count: i64,
    pub artifact_count: i64,
    pub memory_count: i64,
    pub plan_count: i64,
}

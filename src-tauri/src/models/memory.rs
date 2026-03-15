use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemory {
    pub id: String,
    pub content: String,
    pub category: String,
    pub source_chat_id: Option<String>,
    pub source_message_id: Option<String>,
    pub project_id: Option<String>,
    pub is_pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemorySearchResult {
    pub memory: AgentMemory,
    pub score: f64,
}

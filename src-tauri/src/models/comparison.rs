use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbComparison {
    pub id: String,
    pub title: String,
    pub left_provider_id: String,
    pub left_model: String,
    pub left_system_prompt: Option<String>,
    pub right_provider_id: String,
    pub right_model: String,
    pub right_system_prompt: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbComparisonMessage {
    pub id: String,
    pub comparison_id: String,
    pub role: String,
    pub side: Option<String>,
    pub content: String,
    pub timestamp: i64,
    pub model: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost: f64,
}

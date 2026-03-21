use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnAgentConfig {
    pub goal: String,
    pub model: Option<String>,
    pub skills: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub context: Option<String>,
    pub workspace_refs: Option<Vec<String>>,
    pub max_iterations: Option<i64>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubAgentStatus {
    pub agent_run_id: String,
    pub status: String,
    pub iterations: i64,
    pub max_iterations: i64,
    pub error: Option<String>,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubAgentResult {
    pub agent_run_id: String,
    pub status: String,
    pub result: Option<String>,
    pub artifacts_created: Vec<String>,
    pub cost: Option<f64>,
    pub iterations: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubAgentRunInfo {
    pub id: String,
    pub parent_run_id: Option<String>,
    pub status: String,
    pub goal: String,
    pub iterations: i64,
    pub max_iterations: i64,
    pub cost: Option<f64>,
    pub assigned_model: Option<String>,
}

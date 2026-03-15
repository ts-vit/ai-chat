use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlan {
    pub id: String,
    pub chat_id: String,
    pub project_id: Option<String>,
    pub goal: String,
    pub status: String,
    pub execution_mode: String,
    pub replan_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTask {
    pub id: String,
    pub plan_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub dependencies: Vec<String>,
    pub result: Option<String>,
    pub agent_run_id: Option<String>,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWithTasks {
    pub plan: AgentPlan,
    pub tasks: Vec<AgentTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlanWithProgress {
    #[serde(flatten)]
    pub plan: AgentPlan,
    pub total_tasks: i64,
    pub completed_tasks: i64,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub mode: String,
    pub model: Option<String>,
    pub skill_id: Option<String>,
    pub project_id: Option<String>,
    pub deliver_telegram: bool,
    pub deliver_desktop_notification: bool,
    pub last_run_at: Option<i64>,
    pub next_run_at: Option<i64>,
    pub last_run_status: Option<String>,
    pub last_run_error: Option<String>,
    pub run_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatus {
    pub running: bool,
    pub task_count: i32,
    pub next_task_at: Option<i64>,
}

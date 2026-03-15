use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub content: String,
    pub trigger_description: String,
    pub required_tools: String,
    pub is_builtin: bool,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSkill {
    pub chat_id: String,
    pub skill_id: String,
    pub attached_by: String,
    pub created_at: i64,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceArtifact {
    pub id: String,
    pub chat_id: String,
    pub project_id: Option<String>,
    pub name: String,
    pub content_type: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

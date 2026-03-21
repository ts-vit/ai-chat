// Database row / API shapes for chats, messages, folders, etc.
use serde::{Deserialize, Serialize};

// Пресет системного промпта
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbPreset {
    pub id: String,
    pub name: String,
    pub content: String,
    pub is_default: bool,
    pub created_at: i64,
}

fn default_mode() -> String {
    "chat".to_string()
}

// Чат из БД (ответы команд)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbChat {
    pub id: String,
    pub title: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
    #[serde(rename = "systemPrompt", default)]
    pub system_prompt: Option<String>,
    #[serde(rename = "providerId", default)]
    pub provider_id: String,
    #[serde(rename = "model", default)]
    pub model: String,
    #[serde(rename = "folderId", default)]
    pub folder_id: Option<String>,
    #[serde(rename = "projectId", default)]
    pub project_id: Option<String>,
    #[serde(rename = "isImageModel", default)]
    pub is_image_model: bool,
    #[serde(rename = "temperature", skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(rename = "topP", skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(rename = "topK", skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(rename = "frequencyPenalty", skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(rename = "presencePenalty", skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(rename = "imageSize", skip_serializing_if = "Option::is_none")]
    pub image_size: Option<String>,
    #[serde(rename = "imageQuality", skip_serializing_if = "Option::is_none")]
    pub image_quality: Option<String>,
    #[serde(rename = "imageStyle", skip_serializing_if = "Option::is_none")]
    pub image_style: Option<String>,
    #[serde(rename = "imageN", skip_serializing_if = "Option::is_none")]
    pub image_n: Option<u32>,
    #[serde(rename = "negativePrompt", skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(rename = "activeChildMap", default)]
    pub active_child_map: Option<String>,
    #[serde(rename = "mode", default = "default_mode")]
    pub mode: String,
    #[serde(rename = "lastRunStatus", skip_serializing_if = "Option::is_none")]
    pub last_run_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbImageStyle {
    pub id: String,
    pub name: String,
    pub prompt_suffix: String,
    pub is_builtin: bool,
    pub sort_order: i64,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbFolder {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub created_at: i64,
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbCustomProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomProviderInput {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

/// Agent run record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub chat_id: String,
    pub status: String,
    pub iterations: i64,
    pub max_iterations: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_model: Option<String>,
}

// Превью соседней ветки (для выпадающего списка)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiblingPreview {
    pub id: String,
    pub content_preview: String,
    pub created_at: i64,
}

// Сообщение из БД с информацией о ветках (для get_messages_branched)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbMessageWithSiblings {
    #[serde(flatten)]
    pub message: DbMessage,
    pub sibling_count: u32,
    pub sibling_index: u32,
    pub sibling_ids: Vec<String>,
    pub siblings: Vec<SiblingPreview>,
}

// Сообщение из БД (ответы команд)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMessage {
    pub id: String,
    #[serde(rename = "chatId")]
    pub chat_id: String,
    pub role: String,
    pub content: String,
    #[serde(rename = "parentId", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(rename = "promptTokens", skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i64>,
    #[serde(rename = "completionTokens", skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(rename = "hasAttachments", skip_serializing_if = "Option::is_none")]
    pub has_attachments: Option<i64>,
    #[serde(rename = "webSources", skip_serializing_if = "Option::is_none")]
    pub web_sources: Option<String>,
    #[serde(rename = "agentStep", skip_serializing_if = "Option::is_none")]
    pub agent_step: Option<i64>,
    #[serde(rename = "agentRunId", skip_serializing_if = "Option::is_none")]
    pub agent_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbCategory {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbSnippet {
    pub id: String,
    pub name: String,
    pub content: String,
    pub category_id: String,
    pub created_at: i64,
    pub show_on_welcome: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbChatTemplate {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub provider_id: String,
    pub model: String,
    pub system_prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    pub sort_order: i64,
    pub created_at: i64,
}

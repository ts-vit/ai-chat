// структуры данных (Message, ChatRequest, ChatResponse)
use serde::{Deserialize, Serialize};

// Одно сообщение в чате
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,    // "system" | "user" | "assistant"
    pub content: String, // текст сообщения
}

// Запрос к OpenRouter API
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,           // например "anthropic/claude-sonnet-4-20250514"
    pub messages: Vec<Message>,  // история сообщений
    pub stream: bool,            // всегда true для стриминга
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // креативность 0.0-2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,  // лимит длины ответа
}

// Структура SSE-чанка от API (при stream: true)
// Формат: data: {"choices":[{"delta":{"content":"текст"}}]}
#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct StreamResponse {
    pub choices: Vec<StreamChoice>,
    pub usage: Option<Usage>,
}

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
}

// Настройки приложения
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: String,
    pub management_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub font_size: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            management_key: String::new(),
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            font_size: 14,
        }
    }
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
}
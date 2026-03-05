// структуры данных (Message, ChatRequest, ChatResponse)
use serde::{Deserialize, Serialize};

// Одно сообщение в чате
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,    // "system" | "user" | "assistant"
    pub content: String, // текст сообщения
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInput {
    pub name: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_config: Option<serde_json::Value>,
}

// Структура SSE-чанка от API (при stream: true)
// Формат: data: {"choices":[{"delta":{"content":"текст"}}]}
#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

/// Элемент изображения в SSE delta (OpenRouter streaming image generation).
#[derive(Debug, Deserialize)]
pub struct ImageDeltaItem {
    pub image_url: ImageUrl,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<ImageDeltaItem>>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub call_type: Option<String>,
    pub function: Option<FunctionCallDelta>,
}

#[derive(Debug, Deserialize)]
pub struct FunctionCallDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
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
    #[serde(rename = "providerId", default)]
    pub provider_id: String,
    #[serde(rename = "model", default)]
    pub model: String,
    #[serde(rename = "folderId", default)]
    pub folder_id: Option<String>,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbFolder {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub created_at: i64,
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

/// Модель для ответа get_models / get_ollama_models (совместимость с фронтом ModelInfo)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub context_length: u32,
    pub pricing: ModelPricing,
    pub supports_vision: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub prompt: String,
    pub completion: String,
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
    #[serde(default = "default_ollama_url", rename = "ollamaUrl")]
    pub ollama_url: String,
    #[serde(default, rename = "openrouterEnabledModels")]
    pub openrouter_enabled_models: Vec<String>,
    #[serde(default, rename = "ollamaEnabledModels")]
    pub ollama_enabled_models: Vec<String>,
    #[serde(default, rename = "customProviderEnabledModels")]
    pub custom_provider_enabled_models: std::collections::HashMap<String, Vec<String>>,
    #[serde(default, rename = "topP")]
    pub top_p: Option<f32>,
    #[serde(default, rename = "topK")]
    pub top_k: Option<u32>,
    #[serde(default, rename = "frequencyPenalty")]
    pub frequency_penalty: Option<f32>,
    #[serde(default, rename = "presencePenalty")]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    pub language: String,
    #[serde(default = "default_send_by_enter", rename = "sendByEnter")]
    pub send_by_enter: bool,
    #[serde(default, rename = "sttProvider")]
    pub stt_provider: Option<String>,
    #[serde(default, rename = "sttLanguage")]
    pub stt_language: Option<String>,
    #[serde(default, rename = "openaiApiKey")]
    pub openai_api_key: Option<String>,
    #[serde(default, rename = "groqSttApiKey")]
    pub groq_stt_api_key: Option<String>,
    #[serde(default, rename = "ttsProvider")]
    pub tts_provider: Option<String>,
    #[serde(default, rename = "ttsVoice")]
    pub tts_voice: Option<String>,
    #[serde(default, rename = "ttsModel")]
    pub tts_model: Option<String>,
    #[serde(default = "default_message_density", rename = "messageDensity")]
    pub message_density: String,
    #[serde(default = "default_chat_width", rename = "chatWidth")]
    pub chat_width: String,
    #[serde(default = "default_show_status_bar", rename = "showStatusBar")]
    pub show_status_bar: bool,
    #[serde(default = "default_status_bar_metrics", rename = "statusBarMetrics")]
    pub status_bar_metrics: Vec<String>,
}

fn default_ollama_url() -> String {
    "http://localhost:11434/v1".to_string()
}

fn default_send_by_enter() -> bool {
    true
}

fn default_message_density() -> String {
    "standard".to_string()
}

fn default_chat_width() -> String {
    "standard".to_string()
}

fn default_show_status_bar() -> bool {
    true
}

fn default_status_bar_metrics() -> Vec<String> {
    vec![
        "balance".to_string(),
        "context".to_string(),
        "tokens".to_string(),
        "cost".to_string(),
    ]
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
            ollama_url: default_ollama_url(),
            openrouter_enabled_models: Vec::new(),
            ollama_enabled_models: Vec::new(),
            custom_provider_enabled_models: std::collections::HashMap::new(),
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            language: String::new(),
            send_by_enter: true,
            stt_provider: None,
            stt_language: None,
            openai_api_key: None,
            groq_stt_api_key: None,
            tts_provider: None,
            tts_voice: None,
            tts_model: None,
            message_density: default_message_density(),
            chat_width: default_chat_width(),
            show_status_bar: default_show_status_bar(),
            status_bar_metrics: default_status_bar_metrics(),
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
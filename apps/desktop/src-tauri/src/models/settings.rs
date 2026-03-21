// Application settings (store JSON)
use serde::{Deserialize, Serialize};

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
    #[serde(default, rename = "webSearchProvider")]
    pub web_search_provider: Option<String>,
    #[serde(default, rename = "tavilyApiKey")]
    pub tavily_api_key: Option<String>,
    #[serde(default, rename = "braveApiKey")]
    pub brave_api_key: Option<String>,
    #[serde(default, rename = "terminalFontSize")]
    pub terminal_font_size: Option<i32>,
    #[serde(default, rename = "terminalShell")]
    pub terminal_shell: Option<String>,
    #[serde(default, rename = "proxyEnabled")]
    pub proxy_enabled: bool,
    #[serde(default, rename = "proxyType")]
    pub proxy_type: Option<String>,
    #[serde(default, rename = "proxyHost")]
    pub proxy_host: Option<String>,
    #[serde(default, rename = "proxyPort")]
    pub proxy_port: Option<u16>,
    #[serde(default, rename = "proxyUsername")]
    pub proxy_username: Option<String>,
    #[serde(default, rename = "proxyPassword")]
    pub proxy_password: Option<String>,
    #[serde(default, rename = "sshHost")]
    pub ssh_host: Option<String>,
    #[serde(default, rename = "sshPort")]
    pub ssh_port: Option<u16>,
    #[serde(default, rename = "sshUsername")]
    pub ssh_username: Option<String>,
    #[serde(default, rename = "sshAuthType")]
    pub ssh_auth_type: Option<String>,
    #[serde(default, rename = "sshPassword")]
    pub ssh_password: Option<String>,
    #[serde(default, rename = "sshKeyPath")]
    pub ssh_key_path: Option<String>,
    #[serde(default, rename = "sshAutoConnect")]
    pub ssh_auto_connect: bool,
    #[serde(default, rename = "routingEnabled")]
    pub routing_enabled: bool,
    #[serde(default, rename = "routingStrategy")]
    pub routing_strategy: Option<String>,
    #[serde(default, rename = "budgetPlanEnabled")]
    pub budget_plan_enabled: bool,
    #[serde(default, rename = "budgetPlanLimit")]
    pub budget_plan_limit: Option<f64>,
    #[serde(default, rename = "budgetGlobalEnabled")]
    pub budget_global_enabled: bool,
    #[serde(default, rename = "budgetGlobalLimit")]
    pub budget_global_limit: Option<f64>,
    #[serde(default, rename = "budgetGlobalPeriod")]
    pub budget_global_period: Option<String>,
    #[serde(default, rename = "modelCatalogLastSync")]
    pub model_catalog_last_sync: Option<i64>,
    #[serde(default, rename = "telegramBotToken")]
    pub telegram_bot_token: Option<String>,
    #[serde(default, rename = "telegramEnabled")]
    pub telegram_enabled: bool,
    #[serde(default, rename = "telegramAutoStart")]
    pub telegram_auto_start: bool,
    #[serde(default, rename = "telegramModel")]
    pub telegram_model: Option<String>,
    #[serde(default, rename = "embeddingOpenaiKey")]
    pub embedding_openai_key: Option<String>,
    #[serde(default, rename = "embeddingGeminiKey")]
    pub embedding_gemini_key: Option<String>,
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
            web_search_provider: None,
            tavily_api_key: None,
            brave_api_key: None,
            terminal_font_size: None,
            terminal_shell: None,
            proxy_enabled: false,
            proxy_type: None,
            proxy_host: None,
            proxy_port: None,
            proxy_username: None,
            proxy_password: None,
            ssh_host: None,
            ssh_port: None,
            ssh_username: None,
            ssh_auth_type: None,
            ssh_password: None,
            ssh_key_path: None,
            ssh_auto_connect: false,
            routing_enabled: false,
            routing_strategy: None,
            budget_plan_enabled: false,
            budget_plan_limit: None,
            budget_global_enabled: false,
            budget_global_limit: None,
            budget_global_period: None,
            model_catalog_last_sync: None,
            telegram_bot_token: None,
            telegram_enabled: false,
            telegram_auto_start: false,
            telegram_model: None,
            embedding_openai_key: None,
            embedding_gemini_key: None,
        }
    }
}

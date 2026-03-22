// LLM
pub const LLM_PROVIDER: &str = "llm.provider";

// OpenRouter
pub const LLM_OPENROUTER_API_KEY: &str = "llm.openrouter.api_key";
pub const LLM_OPENROUTER_MGMT_KEY: &str = "llm.openrouter.mgmt_key";
pub const LLM_OPENROUTER_MODEL: &str = "llm.openrouter.model";
pub const LLM_OPENROUTER_MODELS: &str = "llm.openrouter.models";

// Ollama
pub const LLM_OLLAMA_URL: &str = "llm.ollama.url";
pub const LLM_OLLAMA_MODEL: &str = "llm.ollama.model";
pub const LLM_OLLAMA_MODELS: &str = "llm.ollama.models";

// OpenAI
pub const LLM_OPENAI_API_KEY: &str = "llm.openai.api_key";
pub const LLM_OPENAI_MODEL: &str = "llm.openai.model";

// Anthropic
pub const LLM_ANTHROPIC_API_KEY: &str = "llm.anthropic.api_key";
pub const LLM_ANTHROPIC_MODEL: &str = "llm.anthropic.model";

// Gemini
pub const LLM_GEMINI_API_KEY: &str = "llm.gemini.api_key";
pub const LLM_GEMINI_MODEL: &str = "llm.gemini.model";

// Grok
pub const LLM_GROK_API_KEY: &str = "llm.grok.api_key";
pub const LLM_GROK_MODEL: &str = "llm.grok.model";

// Custom provider
pub const LLM_CUSTOM_URL: &str = "llm.custom.url";
pub const LLM_CUSTOM_API_KEY: &str = "llm.custom.api_key";
pub const LLM_CUSTOM_MODEL: &str = "llm.custom.model";

// Embedding
pub const EMBEDDING_PROVIDER: &str = "embedding.provider";
pub const EMBEDDING_OPENAI_API_KEY: &str = "embedding.openai.api_key";
pub const EMBEDDING_GEMINI_API_KEY: &str = "embedding.gemini.api_key";

// Web Search
pub const SEARCH_PROVIDER: &str = "search.provider";
pub const SEARCH_TAVILY_API_KEY: &str = "search.tavily.api_key";
pub const SEARCH_BRAVE_API_KEY: &str = "search.brave.api_key";
pub const SEARCH_ENABLED: &str = "search.enabled";

// Audio STT
pub const AUDIO_STT_PROVIDER: &str = "audio.stt.provider";
pub const AUDIO_STT_LANGUAGE: &str = "audio.stt.language";
pub const AUDIO_STT_OPENAI_API_KEY: &str = "audio.stt.openai.api_key";
pub const AUDIO_STT_GROQ_API_KEY: &str = "audio.stt.groq.api_key";

// Audio TTS
pub const AUDIO_TTS_PROVIDER: &str = "audio.tts.provider";
pub const AUDIO_TTS_VOICE: &str = "audio.tts.voice";
pub const AUDIO_TTS_MODEL: &str = "audio.tts.model";

// SSH tunnel
pub const SSH_HOST: &str = "ssh.host";
pub const SSH_PORT: &str = "ssh.port";
pub const SSH_USERNAME: &str = "ssh.username";
pub const SSH_AUTH_TYPE: &str = "ssh.auth_type";
pub const SSH_PASSWORD: &str = "ssh.password";
pub const SSH_KEY_PATH: &str = "ssh.key_path";
pub const SSH_AUTO_CONNECT: &str = "ssh.auto_connect";

// UI
pub const UI_LANGUAGE: &str = "ui.language";
pub const UI_FONT_SIZE: &str = "ui.font_size";
pub const UI_SEND_BY_ENTER: &str = "ui.send_by_enter";
pub const UI_MESSAGE_DENSITY: &str = "ui.message_density";
pub const UI_CHAT_WIDTH: &str = "ui.chat_width";
pub const UI_SHOW_STATUS_BAR: &str = "ui.show_status_bar";
pub const UI_STATUS_BAR_METRICS: &str = "ui.status_bar_metrics";

// LLM parameters
pub const LLM_TEMPERATURE: &str = "llm.temperature";
pub const LLM_MAX_TOKENS: &str = "llm.max_tokens";
pub const LLM_TOP_P: &str = "llm.top_p";
pub const LLM_TOP_K: &str = "llm.top_k";
pub const LLM_FREQUENCY_PENALTY: &str = "llm.frequency_penalty";
pub const LLM_PRESENCE_PENALTY: &str = "llm.presence_penalty";

// Telegram
pub const TELEGRAM_BOT_TOKEN: &str = "telegram.bot_token";
pub const TELEGRAM_ENABLED: &str = "telegram.enabled";
pub const TELEGRAM_AUTO_START: &str = "telegram.auto_start";
pub const TELEGRAM_MODEL: &str = "telegram.model";
pub const TELEGRAM_USER_ID: &str = "telegram.user_id";
pub const TELEGRAM_USER_NAME: &str = "telegram.user_name";

// Routing
pub const ROUTING_ENABLED: &str = "routing.enabled";
pub const ROUTING_STRATEGY: &str = "routing.strategy";

// Budget
pub const BUDGET_PLAN_ENABLED: &str = "budget.plan.enabled";
pub const BUDGET_PLAN_LIMIT: &str = "budget.plan.limit";
pub const BUDGET_GLOBAL_ENABLED: &str = "budget.global.enabled";
pub const BUDGET_GLOBAL_LIMIT: &str = "budget.global.limit";
pub const BUDGET_GLOBAL_PERIOD: &str = "budget.global.period";

// Model catalog
pub const MODEL_CATALOG_LAST_SYNC: &str = "model.catalog.last_sync";

// Reranker
pub const RERANKER_COHERE_API_KEY: &str = "reranker.cohere.api_key";
pub const RERANKER_JINA_API_KEY: &str = "reranker.jina.api_key";

// Terminal
pub const TERMINAL_FONT_SIZE: &str = "terminal.font_size";
pub const TERMINAL_SHELL: &str = "terminal.shell";

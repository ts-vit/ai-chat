pub mod types;
pub mod sse;
pub mod provider;
pub mod completion;

// Реэкспорт основных типов
pub use types::{
    Message, ContentBlock, ImageUrl, AttachmentInput, ChatRequest, StreamChoice, StreamResponse,
    Delta, ToolCallDelta, FunctionCallDelta, ImageDeltaItem, Usage, Model, ModelPricing,
};

// Реэкспорт SSE
pub use sse::{SseParser, SseEvent, ToolCallBuffer};

// Реэкспорт провайдер-хелперов
pub use provider::{build_llm_url, build_llm_headers};

// Реэкспорт completion
pub use completion::{complete, extract_content, extract_usage};

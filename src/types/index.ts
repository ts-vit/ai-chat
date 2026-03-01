// Вложение (черновик на фронте перед отправкой)
export interface Attachment {
    id: string;
    name: string;
    mimeType: string;
    size: number;
    previewUrl: string;
    data: number[];
}

// Одно сообщение в чате
export interface Message {
    id: string;
    role: "system" | "user" | "assistant";
    content: string;
    parentId?: string;
    timestamp: number;
    model?: string;
    promptTokens?: number;
    completionTokens?: number;
    cost?: number;
    hasAttachments?: boolean;
}

// Пресет системного промпта
export interface Preset {
    id: string;
    name: string;
    content: string;
    isDefault: boolean;
    createdAt: number;
}

// Папка для группировки чатов
export interface Folder {
    id: string;
    name: string;
    color: string | null;
    sortOrder: number;
    createdAt: number;
}

// Один чат (беседа)
export interface Chat {
    id: string;
    title: string;
    messages: Message[];
    createdAt: number;
    updatedAt?: number;
    systemPrompt?: string;
    providerId?: string;
    model?: string;
    folderId?: string | null;
}

// Локальная модель Ollama (вывод ollama list)
export interface OllamaLocalModel {
    name: string;
    size: string;
}

// Модель из OpenRouter /api/v1/models
export interface ModelInfo {
    id: string;
    name: string;
    pricing: {
        prompt: string;
        completion: string;
    };
    context_length: number;
    supportsVision: boolean;
    supportsImageGeneration?: boolean;
    description?: string;
}

// Блоки content для сообщений с вложениями (формат в БД)
export interface ContentBlockImage {
    type: "image";
    path: string;
    name: string;
}
export interface ContentBlockFile {
    type: "file";
    path: string;
    name: string;
    mime?: string;
}
export interface ContentBlockText {
    type: "text";
    text: string;
}
export type ContentBlock = ContentBlockImage | ContentBlockFile | ContentBlockText;

// Кастомный провайдер (OpenAI-совместимый API)
export interface CustomProvider {
    id: string;
    name: string;
    baseUrl: string;
    apiKey: string;
    createdAt: number;
}

// Настройки приложения — соответствует Rust AppSettings
export interface AppSettings {
    api_key: string;
    management_key: string;
    model: string;
    temperature: number;
    max_tokens: number;
    topP?: number | null;
    topK?: number | null;
    frequencyPenalty?: number | null;
    presencePenalty?: number | null;
    font_size: number;
    ollamaUrl: string;
    openrouterEnabledModels: string[];
    ollamaEnabledModels: string[];
    customProviderEnabledModels: Record<string, string[]>;
    language: string;
    sendByEnter?: boolean;
}

// Payload событий стриминга — приходят из Rust через emit
export interface StreamPayload {
    content: string;
}

export interface StreamDonePayload {
    full_content: string;
}

export interface StreamErrorPayload {
    error: string;
}

export interface StreamUsagePayload {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
}

export interface StreamImagePayload {
    messageId: string;
    path: string;
    index: number;
}

export interface BalanceInfo {
    total_credits: number;
    total_usage: number;
    remaining: number;
}

export interface Category {
    id: string;
    name: string;
    createdAt: number;
}

export interface Snippet {
    id: string;
    name: string;
    content: string;
    categoryId: string;
    createdAt: number;
}

export interface SearchResult {
    messageId: string;
    chatId: string;
    chatTitle: string;
    content: string;
    role: string;
    timestamp: number;
    score: number;
    source: "semantic" | "fts" | "both";
}

export interface SearchResultGroup {
    chatId: string;
    chatTitle: string;
    results: SearchResult[];
}

export interface IndexingStatus {
    indexed: number;
    total: number;
    inProgress: boolean;
}

export interface ImportResult {
    chatsImported: number;
    messagesImported: number;
    attachmentsImported: number;
}
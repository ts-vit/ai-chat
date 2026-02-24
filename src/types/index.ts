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
}

// Пресет системного промпта
export interface Preset {
    id: string;
    name: string;
    content: string;
    isDefault: boolean;
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
}

// Настройки приложения — соответствует Rust AppSettings
export interface AppSettings {
    api_key: string;
    management_key: string;
    model: string;
    temperature: number;
    max_tokens: number;
    font_size: number;
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
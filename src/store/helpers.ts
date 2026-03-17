import { invoke } from "@tauri-apps/api/core";
import type {
    AppSettings,
    Chat,
    ContentBlock,
    ContentBlockText,
    CustomProvider,
    Message,
    MessageWithSiblings,
    ModeStateMap,
    RagSource,
    WebSource,
} from "../types";

// ── DB response interfaces ─────────────────────────────────────────

export interface DbChatResponse {
    id: string;
    title: string;
    createdAt: number;
    updatedAt: number;
    systemPrompt?: string;
    providerId?: string;
    model?: string;
    folderId?: string | null;
    projectId?: string | null;
    isImageModel?: boolean;
    temperature?: number | null;
    maxTokens?: number | null;
    topP?: number | null;
    topK?: number | null;
    frequencyPenalty?: number | null;
    presencePenalty?: number | null;
    imageSize?: string | null;
    imageQuality?: string | null;
    imageStyle?: string | null;
    imageN?: number | null;
    negativePrompt?: string | null;
    mode?: string;
}

export interface DbMessageResponse {
    id: string;
    chatId: string;
    role: string;
    content: string;
    parentId?: string;
    timestamp: number;
    model?: string;
    promptTokens?: number;
    completionTokens?: number;
    cost?: number;
    has_attachments?: number;
    webSources?: string;
    ragSources?: string;
    agentStep?: number;
    agentRunId?: string;
}

export interface DbMessageWithSiblingsResponse extends DbMessageResponse {
    siblingCount: number;
    siblingIndex: number;
    siblingIds: string[];
    siblings: { id: string; contentPreview: string; createdAt: number }[];
}

// ── Provider resolution ─────────────────────────────────────────

export function resolveProvider(
    providerId: string,
    settings: AppSettings,
    customProviders: CustomProvider[]
): { baseUrl: string; apiKey: string } | null {
    if (providerId === "openrouter") {
        return {
            baseUrl: "https://openrouter.ai/api/v1",
            apiKey: settings.api_key ?? "",
        };
    }
    if (providerId === "ollama") {
        return {
            baseUrl: settings.ollamaUrl?.trim() || "http://localhost:11434/v1",
            apiKey: "",
        };
    }
    const provider = customProviders.find((p) => p.id === providerId);
    if (provider) {
        return { baseUrl: provider.baseUrl, apiKey: provider.apiKey ?? "" };
    }
    return null;
}

// ── Text extraction helpers ─────────────────────────────────────

export function extractTextContent(content: string): string {
    const trimmed = content.trimStart();
    if (!trimmed.startsWith("[")) return content;
    try {
        const blocks = JSON.parse(content) as ContentBlock[];
        if (!Array.isArray(blocks)) return content;
        const textParts = blocks
            .filter((b): b is ContentBlockText => b.type === "text")
            .map((b) => b.text);
        return textParts.join("\n") || "";
    } catch {
        return content;
    }
}

/** Strip markdown/code for TTS: remove code blocks, links, html, keep plain text. */
export function stripMarkdownForTts(content: string): string {
    let text = extractTextContent(content);
    text = text.replace(/```[\s\S]*?```/g, " ");
    text = text.replace(/`[^`]+`/g, " ");
    text = text.replace(/!?\[([^\]]*)\]\([^)]+\)/g, "$1");
    text = text.replace(/<[^>]+>/g, " ");
    text = text.replace(/\*{1,2}([^*]+)\*{1,2}/g, "$1");
    text = text.replace(/_{1,2}([^_]+)_{1,2}/g, "$1");
    text = text.replace(/^#{1,6}\s+/gm, " ");
    return text.replace(/\s+/g, " ").trim();
}

// ── Message mappers ─────────────────────────────────────────

export function parseRagSources(raw: string | undefined | null): RagSource[] | undefined {
    if (!raw) return undefined;
    try {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed as RagSource[];
    } catch { /* ignore */ }
    return undefined;
}

export function parseWebSources(raw: string | undefined | null): WebSource[] | undefined {
    if (!raw) return undefined;
    try {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed as WebSource[];
    } catch { /* ignore */ }
    return undefined;
}

export function dbMessageToMessage(m: DbMessageResponse): Message {
    return {
        id: m.id,
        role: m.role as Message["role"],
        content: m.content,
        parentId: m.parentId,
        timestamp: m.timestamp,
        model: m.model,
        promptTokens: m.promptTokens,
        completionTokens: m.completionTokens,
        cost: m.cost,
        hasAttachments: m.has_attachments ? true : undefined,
        webSources: parseWebSources(m.webSources),
        ragSources: parseRagSources(m.ragSources),
        agentStep: m.agentStep,
        agentRunId: m.agentRunId,
    };
}

export function dbMessageToMessageWithSiblings(m: DbMessageWithSiblingsResponse): MessageWithSiblings {
    return {
        ...dbMessageToMessage(m),
        siblingCount: m.siblingCount,
        siblingIndex: m.siblingIndex,
        siblingIds: m.siblingIds,
        siblings: m.siblings || [],
    };
}

export function toMessageWithSiblings(m: Message): MessageWithSiblings {
    return { ...m, siblingCount: 1, siblingIndex: 0, siblingIds: [m.id], siblings: [] };
}

// ── Reload messages helper ──────────────────────────────────────

/** Generation counter per chat — prevents stale reloads from overwriting fresh data */
const _reloadGen: Record<string, number> = {};

// Use a generic type for the set/get functions to avoid circular imports.
// The actual StoreState type will be used once all slices are in place.

/** Reload messages from DB for a given chat and update store */
export async function reloadChatMessages(
    chatId: string,
    _get: () => { chats: Chat[] },
    set: (partial: Partial<{ chats: Chat[] }> | ((state: { chats: Chat[] }) => Partial<{ chats: Chat[] }>)) => void,
) {
    const gen = (_reloadGen[chatId] = (_reloadGen[chatId] ?? 0) + 1);
    try {
        const rows = await invoke<DbMessageWithSiblingsResponse[]>("get_messages_branched", { chatId });
        if (_reloadGen[chatId] !== gen) return;
        const messages = rows.map(dbMessageToMessageWithSiblings);
        set((state) => ({
            chats: state.chats.map((c) => (c.id === chatId ? { ...c, messages } : c)),
        }));
    } catch (e) {
        console.error("[reloadChatMessages] failed:", e);
    }
}

// ── Mode init vars ──────────────────────────────────────────────

export const _initModeState: ModeStateMap = JSON.parse(localStorage.getItem('uni-mode-state') ?? 'null') ?? {
    chat: { activeChatId: null, activeComparisonId: null },
    assistant: { activeChatId: null, activeComparisonId: null },
};

export const _initActiveMode = localStorage.getItem('uni-active-mode') ?? 'chat';

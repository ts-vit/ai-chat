import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import i18n from "../i18n";
import { notify } from "../utils/notify";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
    Attachment,
    Chat,
    Folder,
    Message,
    Preset,
    Category,
    Snippet,
    AppSettings,
    CustomProvider,
    ModelInfo,
    OllamaLocalModel,
    BalanceInfo,
    StreamPayload,
    StreamDonePayload,
    StreamErrorPayload,
    StreamUsagePayload,
    StreamImagePayload,
    ContentBlock,
} from "../types";
import { toUnixSeconds } from "../utils/formatDate";

interface DbChatResponse {
    id: string;
    title: string;
    createdAt: number;
    updatedAt: number;
    systemPrompt?: string;
    providerId?: string;
    model?: string;
    folderId?: string | null;
    isImageModel?: boolean;
}

function resolveProvider(
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

function extractTextContent(content: string): string {
    const trimmed = content.trimStart();
    if (!trimmed.startsWith("[")) return content;
    try {
        const blocks = JSON.parse(content) as ContentBlock[];
        if (!Array.isArray(blocks)) return content;
        const textParts = blocks
            .filter((b) => b.type === "text" && b.text)
            .map((b) => b.text!);
        return textParts.join("\n") || "";
    } catch {
        return content;
    }
}

interface DbMessageResponse {
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
}

function dbMessageToMessage(m: DbMessageResponse): Message {
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
    };
}

interface ChatState {
    chats: Chat[];
    activeChatId: string | null;
    settings: AppSettings;
    presets: Preset[];
    folders: Folder[];
    categories: Category[];
    snippets: Snippet[];
    customProviders: CustomProvider[];
    insertSnippetText: string | null;
    isStreaming: boolean;
    isStopping: boolean;
    balance: BalanceInfo | null;
    draftAttachments: Attachment[];

    currentView: "chat" | "settings" | "snippets" | "search";
    setView: (view: "chat" | "settings" | "snippets" | "search") => void;
    scrollTargetId: string | null;
    setScrollTargetId: (id: string | null) => void;
    models: ModelInfo[];
    ollamaStatus: "unknown" | "available" | "unavailable";
    localOllamaModels: OllamaLocalModel[];
    checkOllamaStatus: () => Promise<void>;
    loadLocalOllamaModels: () => Promise<void>;
    deleteOllamaModel: (modelName: string) => Promise<void>;
    modelsLoading: boolean;
    modelsError: string | null;

    loadCustomProviders: () => Promise<void>;
    createCustomProvider: (name: string, baseUrl: string, apiKey: string) => Promise<void>;
    updateCustomProvider: (id: string, name: string, baseUrl: string, apiKey: string) => Promise<void>;
    deleteCustomProvider: (id: string) => Promise<void>;
    testOllamaConnection: (baseUrl: string) => Promise<number>;
    addAttachment: (attachment: Attachment) => void;
    removeAttachment: (id: string) => void;
    clearAttachments: () => void;
    loadModels: (force?: boolean) => Promise<void>;
    loadBalance: () => Promise<void>;
    loadPresets: () => Promise<void>;
    createPreset: (name: string, content: string, isDefault: boolean) => Promise<void>;
    updatePreset: (id: string, name: string, content: string, isDefault: boolean) => Promise<void>;
    deletePreset: (id: string) => Promise<void>;
    setChatSystemPrompt: (chatId: string, systemPrompt: string) => Promise<void>;
    updateChatModel: (chatId: string, model: string, isImageModel?: boolean) => Promise<void>;

    loadCategories: () => Promise<void>;
    createCategory: (name: string) => Promise<void>;
    updateCategory: (id: string, name: string) => Promise<void>;
    deleteCategory: (id: string) => Promise<void>;
    loadSnippets: () => Promise<void>;
    createSnippet: (name: string, content: string, categoryId: string) => Promise<void>;
    updateSnippet: (id: string, name: string, content: string, categoryId: string) => Promise<void>;
    deleteSnippet: (id: string) => Promise<void>;
    setInsertSnippetText: (text: string | null) => void;

    createChat: (providerId?: string, model?: string, isImageModel?: boolean) => Promise<void>;
    deleteChat: (id: string) => Promise<void>;
    setActiveChat: (id: string) => Promise<void>;
    loadChats: () => Promise<void>;
    loadFolders: () => Promise<void>;
    createFolder: (name: string, color: string | null) => Promise<void>;
    updateFolder: (id: string, name: string, color: string | null) => Promise<void>;
    deleteFolder: (id: string) => Promise<void>;
    reorderFolders: (ids: string[]) => Promise<void>;
    moveChatToFolder: (chatId: string, folderId: string | null) => Promise<void>;

    sendMessage: (content: string) => Promise<void>;
    editAndResend: (messageId: string, newContent: string) => Promise<void>;
    stopGeneration: () => Promise<void>;

    loadSettings: () => Promise<void>;
    saveSettings: (settings: AppSettings) => Promise<void>;
}

export const useChatStore = create<ChatState>((set, get) => ({
    chats: [],
    activeChatId: null,
    presets: [],
    folders: [],
    categories: [],
    snippets: [],
    customProviders: [],
    insertSnippetText: null,
    settings: {
        api_key: "",
        management_key: "",
        model: "anthropic/claude-sonnet-4-20250514",
        temperature: 0.7,
        max_tokens: 4096,
        font_size: 14,
        ollamaUrl: "http://localhost:11434/v1",
        openrouterEnabledModels: [],
        ollamaEnabledModels: [],
        customProviderEnabledModels: {},
        language: "",
        sendByEnter: true,
    },
    isStreaming: false,
    isStopping: false,
    balance: null,
    draftAttachments: [],

    addAttachment: (attachment) =>
        set((state) => ({ draftAttachments: [...state.draftAttachments, attachment] })),
    removeAttachment: (id) =>
        set((state) => ({ draftAttachments: state.draftAttachments.filter((a) => a.id !== id) })),
    clearAttachments: () => set({ draftAttachments: [] }),

    currentView: "chat",
    setView: (view) => set({ currentView: view }),
    scrollTargetId: null,
    setScrollTargetId: (id) => set({ scrollTargetId: id }),
    models: [],
    modelsLoading: false,
    modelsError: null,
    ollamaStatus: "unknown",
    localOllamaModels: [],
    checkOllamaStatus: async () => {
        set({ ollamaStatus: "unknown" });
        try {
            const ok = await invoke<boolean>("check_ollama_status");
            set({ ollamaStatus: ok ? "available" : "unavailable" });
        } catch {
            set({ ollamaStatus: "unavailable" });
        }
    },
    loadLocalOllamaModels: async () => {
        try {
            const list = await invoke<OllamaLocalModel[]>("get_local_ollama_models");
            set({ localOllamaModels: list ?? [] });
        } catch {
            set({ localOllamaModels: [] });
        }
    },
    deleteOllamaModel: async (modelName) => {
        try {
            await invoke("delete_ollama_model", { modelName });
            await get().loadLocalOllamaModels();
            notify.success(i18n.t("notifications.modelDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },
    loadModels: async (force) => {
        const { models } = get();
        if (!force && models.length > 0) return;
        set({ modelsLoading: true, modelsError: null, ...(force ? { models: [] } : {}) });
        try {
            const response = await invoke<{ data: unknown }>("get_models");
            const raw = response?.data;
            if (!Array.isArray(raw)) {
                set({ models: [], modelsLoading: false, modelsError: i18n.t("notifications.modelsLoadError") });
                return;
            }
            interface ModelRaw {
                id: string;
                name: string;
                context_length: number;
                pricing: { prompt: string; completion: string };
                architecture?: { modality?: string; output_modalities?: string[] };
                output_modalities?: string[];
                description?: string;
            }
            const list: ModelInfo[] = raw
                .filter(
                    (m: unknown): m is ModelRaw =>
                        typeof m === "object" &&
                        m !== null &&
                        typeof (m as ModelRaw).id === "string" &&
                        typeof (m as ModelRaw).name === "string" &&
                        typeof (m as ModelRaw).context_length === "number" &&
                        typeof (m as ModelRaw).pricing === "object" &&
                        (m as ModelRaw).pricing !== null &&
                        typeof (m as ModelRaw).pricing.prompt === "string" &&
                        typeof (m as ModelRaw).pricing.completion === "string"
                )
                .map((m) => {
                    const modality = m.architecture?.modality;
                    const outputModalities =
                        m.architecture?.output_modalities ?? m.output_modalities ?? [];
                    const supportsImageGeneration = Array.isArray(outputModalities)
                        ? outputModalities.includes("image")
                        : false;
                    return {
                        id: m.id,
                        name: m.name,
                        pricing: { prompt: m.pricing.prompt, completion: m.pricing.completion },
                        context_length: m.context_length,
                        supportsVision: modality?.includes("image") ?? false,
                        supportsImageGeneration,
                        description:
                            typeof (m as ModelRaw).description === "string"
                                ? (m as ModelRaw).description
                                : undefined,
                    };
                });
            set({ models: list, modelsLoading: false, modelsError: null });
        } catch (e) {
            set({ modelsLoading: false, modelsError: String(e) });
            notify.error(i18n.t("notifications.modelsLoadError"));
        }
    },

    loadPresets: async () => {
        try {
            const list = await invoke<Preset[]>("get_all_presets");
            set({ presets: list ?? [] });
        } catch (e) {
            console.error("Failed to load presets:", e);
        }
    },

    createPreset: async (name, content, isDefault) => {
        try {
            const preset = await invoke<Preset>("create_preset", {
                name,
                content,
                isDefault,
            });
            set((state) => ({ presets: [...state.presets, preset] }));
            notify.success(i18n.t("notifications.presetCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updatePreset: async (id, name, content, isDefault) => {
        try {
            await invoke("update_preset", { id, name, content, isDefault });
            set((state) => ({
                presets: state.presets.map((p) =>
                    p.id === id ? { ...p, name, content, isDefault } : p
                ),
            }));
            notify.success(i18n.t("notifications.presetUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deletePreset: async (id) => {
        try {
            await invoke("delete_preset", { id });
            set((state) => ({ presets: state.presets.filter((p) => p.id !== id) }));
            notify.success(i18n.t("notifications.presetDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    setChatSystemPrompt: async (chatId, systemPrompt) => {
        try {
            await invoke("set_chat_system_prompt", {
                chatId,
                systemPrompt,
            });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, systemPrompt: systemPrompt || undefined } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateChatModel: async (chatId, model, isImageModel = false) => {
        try {
            await invoke("update_chat_model", { chatId, model, isImageModel });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, model } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    loadCategories: async () => {
        try {
            const list = await invoke<Category[]>("get_all_categories");
            set({ categories: list ?? [] });
        } catch (e) {
            console.error("Failed to load categories:", e);
        }
    },

    createCategory: async (name) => {
        try {
            const category = await invoke<Category>("create_category", { name });
            set((state) => ({ categories: [...state.categories, category] }));
            notify.success(i18n.t("notifications.categoryCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateCategory: async (id, name) => {
        try {
            await invoke("update_category", { id, name });
            set((state) => ({
                categories: state.categories.map((c) =>
                    c.id === id ? { ...c, name } : c
                ),
            }));
            notify.success(i18n.t("notifications.categoryUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteCategory: async (id) => {
        try {
            await invoke("delete_category", { id });
            set((state) => ({ categories: state.categories.filter((c) => c.id !== id) }));
            notify.success(i18n.t("notifications.categoryDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    loadSnippets: async () => {
        try {
            const list = await invoke<Snippet[]>("get_all_snippets");
            set({ snippets: list ?? [] });
        } catch (e) {
            console.error("Failed to load snippets:", e);
        }
    },

    createSnippet: async (name, content, categoryId) => {
        try {
            const snippet = await invoke<Snippet>("create_snippet", {
                name,
                content,
                categoryId,
            });
            set((state) => ({ snippets: [...state.snippets, snippet] }));
            notify.success(i18n.t("notifications.snippetCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateSnippet: async (id, name, content, categoryId) => {
        try {
            await invoke("update_snippet", { id, name, content, categoryId });
            set((state) => ({
                snippets: state.snippets.map((s) =>
                    s.id === id ? { ...s, name, content, categoryId } : s
                ),
            }));
            notify.success(i18n.t("notifications.snippetUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteSnippet: async (id) => {
        try {
            await invoke("delete_snippet", { id });
            set((state) => ({ snippets: state.snippets.filter((s) => s.id !== id) }));
            notify.success(i18n.t("notifications.snippetDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    loadCustomProviders: async () => {
        try {
            const list = await invoke<CustomProvider[]>("get_custom_providers");
            set({ customProviders: list ?? [] });
        } catch (e) {
            console.error("Failed to load custom providers:", e);
        }
    },

    createCustomProvider: async (name, baseUrl, apiKey) => {
        try {
            await invoke("create_custom_provider", { name, baseUrl, apiKey });
            await get().loadCustomProviders();
            notify.success(i18n.t("notifications.providerAdded"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateCustomProvider: async (id, name, baseUrl, apiKey) => {
        try {
            await invoke("update_custom_provider", {
                id,
                input: { name, baseUrl, apiKey },
            });
            await get().loadCustomProviders();
            notify.success(i18n.t("notifications.providerUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteCustomProvider: async (id) => {
        try {
            await invoke("delete_custom_provider", { id });
            await get().loadCustomProviders();
            notify.success(i18n.t("notifications.providerDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    testOllamaConnection: async (baseUrl) => {
        const list = await invoke<unknown[]>("get_ollama_models", { baseUrl });
        return Array.isArray(list) ? list.length : 0;
    },

    setInsertSnippetText: (text) => set({ insertSnippetText: text }),

    loadBalance: async () => {
        const { settings } = get();
        if (!settings.management_key?.trim()) {
            set({ balance: null });
            return;
        }
        try {
            const response = await invoke<{
                data?: { total_credits?: number; total_usage?: number };
            }>("get_credits", {
                managementKey: settings.management_key,
            });
            const data = response?.data;
            const totalCredits = data?.total_credits;
            const totalUsage = data?.total_usage;
            if (
                typeof totalCredits === "number" &&
                typeof totalUsage === "number"
            ) {
                const remaining = totalCredits - totalUsage;
                set({
                    balance: {
                        total_credits: totalCredits,
                        total_usage: totalUsage,
                        remaining,
                    },
                });
            } else {
                set({ balance: null });
            }
        } catch {
            set({ balance: null });
        }
    },

    createChat: async (providerId = "openrouter", model = "", isImageModel = false) => {
        try {
            const { presets } = get();
            const defaultPreset = presets.find((p) => p.isDefault);
            const systemPrompt = defaultPreset?.content ?? "";

            const created = await invoke<DbChatResponse>("create_chat", {
                title: i18n.t("chat.newChatTitle"),
                systemPrompt,
                providerId,
                model,
                isImageModel,
            });
            const newChat: Chat = {
                id: created.id,
                title: created.title,
                messages: [],
                createdAt: created.createdAt,
                updatedAt: created.updatedAt,
                systemPrompt: systemPrompt || undefined,
                providerId: created.providerId ?? "openrouter",
                model: created.model ?? "",
                folderId: null,
            };
            set((state) => ({
                chats: [newChat, ...state.chats],
                activeChatId: newChat.id,
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteChat: async (id: string) => {
        try {
            await invoke("delete_search_index", { chatId: id }).catch(console.error);
            await invoke("delete_chat", { id });
            set((state) => {
                const chats = state.chats.filter((c) => c.id !== id);
                const activeChatId =
                    state.activeChatId === id
                        ? chats.length > 0
                            ? chats[0].id
                            : null
                        : state.activeChatId;
                return { chats, activeChatId };
            });
            notify.info(i18n.t("notifications.chatDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    setActiveChat: async (id: string) => {
        set({ activeChatId: id, draftAttachments: [] });
        const { chats } = get();
        const chat = chats.find((c) => c.id === id);
        if (chat && chat.messages.length === 0) {
            try {
                const rows = await invoke<DbMessageResponse[]>("get_messages", {
                    chatId: id,
                });
                const messages = rows.map(dbMessageToMessage);
                set((state) => ({
                    chats: state.chats.map((c) =>
                        c.id === id ? { ...c, messages } : c
                    ),
                }));
            } catch (e) {
                console.error("Failed to load messages:", e);
            }
        }
    },

    loadChats: async () => {
        try {
            const list = await invoke<DbChatResponse[]>("get_all_chats");
            const chats: Chat[] = list.map((c) => ({
                id: c.id,
                title: c.title,
                messages: [],
                createdAt: c.createdAt,
                updatedAt: c.updatedAt,
                systemPrompt: c.systemPrompt || undefined,
                providerId: c.providerId ?? "openrouter",
                model: c.model ?? "",
                folderId: c.folderId ?? null,
            }));
            set({ chats });
        } catch (e) {
            console.error("Failed to load chats:", e);
        }
    },

    loadFolders: async () => {
        try {
            const list = await invoke<Folder[]>("get_all_folders");
            set({ folders: list ?? [] });
        } catch (e) {
            console.error("Failed to load folders:", e);
        }
    },

    createFolder: async (name: string, color: string | null) => {
        try {
            const folder = await invoke<Folder>("create_folder", { name, color });
            set((state) => ({ folders: [...state.folders, folder] }));
            notify.success(i18n.t("notifications.folderCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateFolder: async (id: string, name: string, color: string | null) => {
        try {
            await invoke("update_folder", { id, name, color });
            set((state) => ({
                folders: state.folders.map((f) =>
                    f.id === id ? { ...f, name, color } : f
                ),
            }));
            notify.success(i18n.t("notifications.folderUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteFolder: async (id: string) => {
        try {
            await invoke("delete_folder", { id });
            set((state) => ({ folders: state.folders.filter((f) => f.id !== id) }));
            notify.success(i18n.t("notifications.folderDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    reorderFolders: async (ids: string[]) => {
        try {
            await invoke("reorder_folders", { folderIds: ids });
            set((state) => {
                const byId = new Map(state.folders.map((f) => [f.id, f]));
                const reordered = ids
                    .map((id, index) => {
                        const f = byId.get(id);
                        return f ? { ...f, sortOrder: index } : null;
                    })
                    .filter((f): f is Folder => f !== null);
                const remaining = state.folders.filter((f) => !byId.has(f.id));
                return { folders: [...reordered, ...remaining] };
            });
        } catch (e) {
            notify.error(String(e));
        }
    },

    moveChatToFolder: async (chatId: string, folderId: string | null) => {
        try {
            await invoke("move_chat_to_folder", { chatId, folderId });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, folderId } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    sendMessage: async (content: string) => {
        const { settings, activeChatId, chats, customProviders } = get();

        if (!activeChatId) return;

        const chat = chats.find((c) => c.id === activeChatId);
        if (!chat) return;

        const providerId = chat.providerId ?? "openrouter";
        const resolved = resolveProvider(providerId, settings, customProviders);
        if (!resolved) {
            notify.error(i18n.t("notifications.providerNotFound"));
            return;
        }
        if (providerId === "openrouter" && !resolved.apiKey?.trim()) {
            notify.warning(i18n.t("notifications.apiKeyNotSet"));
            return;
        }

        const now = Math.floor(Date.now() / 1000);

        try {
            const userMsg = await invoke<DbMessageResponse>("save_message", {
                chatId: activeChatId,
                role: "user",
                content,
                parentId: null,
                timestamp: now,
                model: "",
                promptTokens: 0,
                completionTokens: 0,
                cost: 0.0,
            });
            const chatModel = chat.model ?? "";
            const assistantMsg = await invoke<DbMessageResponse>("save_message", {
                chatId: activeChatId,
                role: "assistant",
                content: "",
                parentId: userMsg.id,
                timestamp: now + 1,
                model: chatModel,
                promptTokens: 0,
                completionTokens: 0,
                cost: 0.0,
            });
            const userMessage: Message = dbMessageToMessage(userMsg);
            const assistantMessage: Message = {
                ...dbMessageToMessage(assistantMsg),
                content: "",
                model: chatModel,
            };
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === activeChatId
                        ? { ...c, messages: [...c.messages, userMessage, assistantMessage] }
                        : c
                ),
                isStreaming: true,
            }));

            if (chat.title === i18n.t("chat.newChatTitle") && chat.messages.length === 0) {
                const trimmed = content.trim();
                if (trimmed) {
                    const title = trimmed.substring(0, 60);
                    const truncated =
                        title.length < trimmed.length
                            ? (title.lastIndexOf(" ") > 0
                                  ? title.substring(0, title.lastIndexOf(" "))
                                  : title) + "…"
                            : title;
                    try {
                        await invoke("update_chat_title", {
                            chatId: activeChatId,
                            title: truncated,
                        });
                        set((state) => ({
                            chats: state.chats.map((c) =>
                                c.id === activeChatId ? { ...c, title: truncated } : c
                            ),
                        }));
                    } catch (e) {
                        console.error("update_chat_title failed:", e);
                    }
                }
            }

            const unlisteners: UnlistenFn[] = [];
            const streamImagePaths: { path: string; index: number }[] = [];

            unlisteners.push(
                await listen<StreamPayload>("chat-stream", (event) => {
                    set((state) => {
                        const chat = state.chats.find((c) => c.id === activeChatId);
                        if (!chat) return state;
                        const lastMsg = chat.messages[chat.messages.length - 1];
                        if (!lastMsg || lastMsg.id !== assistantMsg.id) return state;
                        let newContent: string;
                        const trimmed = lastMsg.content.trimStart();
                        if (trimmed.startsWith("[")) {
                            try {
                                const blocks = JSON.parse(lastMsg.content) as ContentBlock[];
                                const textBlock = blocks.find((b) => b.type === "text");
                                if (textBlock && textBlock.type === "text") {
                                    const updated = blocks.map((b) =>
                                        b.type === "text"
                                            ? { ...b, text: b.text + event.payload.content }
                                            : b
                                    );
                                    newContent = JSON.stringify(updated);
                                } else {
                                    newContent = lastMsg.content + event.payload.content;
                                }
                            } catch {
                                newContent = lastMsg.content + event.payload.content;
                            }
                        } else {
                            newContent = lastMsg.content + event.payload.content;
                        }
                        return {
                            chats: state.chats.map((c) =>
                                c.id === activeChatId
                                    ? {
                                          ...c,
                                          messages: c.messages.map((msg) =>
                                              msg.id === assistantMsg.id
                                                  ? { ...msg, content: newContent }
                                                  : msg
                                          ),
                                      }
                                    : c
                            ),
                        };
                    });
                })
            );

            unlisteners.push(
                await listen<StreamImagePayload>("chat-stream-image", (event) => {
                    const payload = event.payload;
                    if (payload.messageId !== assistantMsg.id) return;
                    streamImagePaths.push({ path: payload.path, index: payload.index });
                    const imageBlock: ContentBlock = {
                        type: "image",
                        path: payload.path,
                        name: `image_${payload.index}.png`,
                    };
                    set((state) => {
                        const chat = state.chats.find((c) => c.id === activeChatId);
                        if (!chat) return state;
                        const msg = chat.messages.find((m) => m.id === assistantMsg.id);
                        if (!msg) return state;
                        let blocks: ContentBlock[];
                        const trimmed = msg.content.trimStart();
                        if (trimmed.startsWith("[")) {
                            try {
                                blocks = JSON.parse(msg.content) as ContentBlock[];
                            } catch {
                                blocks = [{ type: "text", text: msg.content }];
                            }
                            blocks = [...blocks, imageBlock];
                        } else {
                            blocks = [{ type: "text", text: msg.content }, imageBlock];
                        }
                        return {
                            chats: state.chats.map((c) =>
                                c.id === activeChatId
                                    ? {
                                          ...c,
                                          messages: c.messages.map((m) =>
                                              m.id === assistantMsg.id
                                                  ? { ...m, content: JSON.stringify(blocks) }
                                                  : m
                                          ),
                                      }
                                    : c
                            ),
                        };
                    });
                })
            );

            unlisteners.push(
                await listen<StreamUsagePayload>("chat-stream-usage", (event) => {
                    const { models, chats } = get();
                    const payload = event.payload;
                    const currentChat = chats.find((c) => c.id === activeChatId);
                    const modelId = currentChat?.model ?? "";
                    const modelInfo = modelId ? models.find((m) => m.id === modelId) : null;
                    let cost = 0;
                    if (modelInfo?.pricing) {
                        cost =
                            payload.prompt_tokens * parseFloat(modelInfo.pricing.prompt) +
                            payload.completion_tokens * parseFloat(modelInfo.pricing.completion);
                    }
                    invoke("update_message_usage", {
                        id: assistantMsg.id,
                        promptTokens: payload.prompt_tokens,
                        completionTokens: payload.completion_tokens,
                        cost,
                    }).catch(console.error);
                    set((state) => ({
                        chats: state.chats.map((c) =>
                            c.id === activeChatId
                                ? {
                                      ...c,
                                      messages: c.messages.map((msg) =>
                                          msg.id === assistantMsg.id
                                              ? {
                                                    ...msg,
                                                    promptTokens: payload.prompt_tokens,
                                                    completionTokens: payload.completion_tokens,
                                                    cost,
                                                }
                                              : msg
                                      ),
                                  }
                                : c
                        ),
                    }));
                })
            );

            unlisteners.push(
                await listen<StreamDonePayload>("chat-stream-done", (event) => {
                    const fullContent = event.payload.full_content;
                    let contentToSave: string;
                    if (streamImagePaths.length > 0) {
                        const sorted = [...streamImagePaths].sort((a, b) => a.index - b.index);
                        const blocks: ContentBlock[] = [
                            { type: "text", text: fullContent },
                            ...sorted.map((p) => ({
                                type: "image" as const,
                                path: p.path,
                                name: `image_${p.index}.png`,
                            })),
                        ];
                        contentToSave = JSON.stringify(blocks);
                    } else {
                        contentToSave = fullContent;
                    }
                    invoke("update_message_content", {
                        id: assistantMsg.id,
                        content: contentToSave,
                    }).catch(console.error);
                    set((state) => ({
                        isStreaming: false,
                        isStopping: false,
                        chats: state.chats.map((c) =>
                            c.id === activeChatId
                                ? {
                                      ...c,
                                      messages: c.messages.map((msg) =>
                                          msg.id === assistantMsg.id
                                              ? { ...msg, content: contentToSave }
                                              : msg
                                      ),
                                  }
                                : c
                        ),
                    }));
                    unlisteners.forEach((fn) => fn());
                    get().loadBalance();
                    invoke("index_message", { messageId: userMsg.id }).catch(console.error);
                    invoke("index_message", { messageId: assistantMsg.id }).catch(console.error);
                })
            );

            unlisteners.push(
                await listen<StreamErrorPayload>("chat-stream-error", (event) => {
                    notify.error(event.payload.error);
                    set({ isStreaming: false, isStopping: false });
                    unlisteners.forEach((fn) => fn());
                })
            );

            const history = chat.messages
                .filter((m) => m.role !== "assistant" || m.content)
                .map((m) => ({ role: m.role, content: extractTextContent(m.content) }));
            const apiMessages = [
                ...(chat.systemPrompt?.trim()
                    ? [{ role: "system" as const, content: chat.systemPrompt }]
                    : []),
                ...history,
                { role: "user" as const, content },
            ];
            const { draftAttachments } = get();
            const invokePayload: Record<string, unknown> = {
                chatId: activeChatId,
                baseUrl: resolved.baseUrl,
                apiKey: resolved.apiKey,
                model: chat.model ?? "",
                messages: apiMessages,
                temperature: settings.temperature,
                maxTokens: settings.max_tokens,
                topP: settings.topP ?? null,
                topK: settings.topK ?? null,
                frequencyPenalty: settings.frequencyPenalty ?? null,
                presencePenalty: settings.presencePenalty ?? null,
                assistantMessageId: assistantMsg.id,
            };
            if (draftAttachments.length > 0) {
                invokePayload.userMessageId = userMsg.id;
                invokePayload.attachments = draftAttachments.map((a) => ({
                    name: a.name,
                    mimeType: a.mimeType,
                    data: a.data,
                }));
            }
            const hadAttachments = get().draftAttachments.length > 0;
            await invoke("send_message", invokePayload);
            set({ draftAttachments: [] });
            if (hadAttachments) {
                try {
                    const rows = await invoke<DbMessageResponse[]>("get_messages", {
                        chatId: activeChatId,
                    });
                    const updatedUser = rows.find((m) => m.id === userMsg.id);
                    if (updatedUser) {
                        set((state) => ({
                            chats: state.chats.map((c) =>
                                c.id === activeChatId
                                    ? {
                                          ...c,
                                          messages: c.messages.map((msg) =>
                                              msg.id === userMsg.id
                                                  ? {
                                                        ...msg,
                                                        content: updatedUser.content,
                                                        hasAttachments: updatedUser.has_attachments
                                                            ? true
                                                            : undefined,
                                                    }
                                                  : msg
                                          ),
                                    }
                                : c
                            ),
                        }));
                    }
                } catch {
                    // не обновляем content в сторе при ошибке get_messages
                }
            }
        } catch (e) {
            set({ isStreaming: false });
            notify.error(String(e));
        }
    },

    editAndResend: async (messageId: string, newContent: string) => {
        const { settings, activeChatId, chats, customProviders, isStreaming } = get();

        if (isStreaming) return;
        if (!activeChatId) return;

        const chat = chats.find((c) => c.id === activeChatId);
        if (!chat) return;

        const providerId = chat.providerId ?? "openrouter";
        const resolved = resolveProvider(providerId, settings, customProviders);
        if (!resolved) {
            notify.error(i18n.t("notifications.providerNotFound"));
            return;
        }
        if (providerId === "openrouter" && !resolved.apiKey?.trim()) {
            notify.warning(i18n.t("notifications.apiKeyNotSet"));
            return;
        }

        const idx = chat.messages.findIndex((m) => m.id === messageId);
        if (idx === -1) return;

        const msg = chat.messages[idx];
        if (msg.role !== "user") return;

        try {
            const newMessages = chat.messages.slice(0, idx + 1).map((m, i) =>
                i === idx ? { ...m, content: newContent } : m
            );

            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === activeChatId ? { ...c, messages: newMessages } : c
                ),
            }));

            await invoke("update_message_content", { id: messageId, content: newContent });
            await invoke("delete_messages_after", {
                chatId: activeChatId,
                timestamp: msg.timestamp,
            });

            const { chats: chatsAfter } = get();
            const chatAfter = chatsAfter.find((c) => c.id === activeChatId);
            if (!chatAfter) return;

            const history = chatAfter.messages
                .filter((m) => m.role !== "assistant" || m.content)
                .map((m) => ({ role: m.role, content: extractTextContent(m.content) }));
            const apiMessages = [
                ...(chatAfter.systemPrompt?.trim()
                    ? [{ role: "system" as const, content: chatAfter.systemPrompt }]
                    : []),
                ...history,
            ];

            const chatModel = chat.model ?? "";
            const assistantMsg = await invoke<DbMessageResponse>("save_message", {
                chatId: activeChatId,
                role: "assistant",
                content: "",
                parentId: messageId,
                timestamp: toUnixSeconds(msg.timestamp) + 1,
                model: chatModel,
                promptTokens: 0,
                completionTokens: 0,
                cost: 0.0,
            });

            const assistantMessage: Message = {
                ...dbMessageToMessage(assistantMsg),
                content: "",
                model: chatModel,
            };

            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === activeChatId
                        ? { ...c, messages: [...c.messages, assistantMessage] }
                        : c
                ),
                isStreaming: true,
            }));

            const unlisteners: UnlistenFn[] = [];
            const streamImagePathsEdit: { path: string; index: number }[] = [];

            unlisteners.push(
                await listen<StreamPayload>("chat-stream", (event) => {
                    set((state) => {
                        const chat = state.chats.find((c) => c.id === activeChatId);
                        if (!chat) return state;
                        const lastMsg = chat.messages[chat.messages.length - 1];
                        if (!lastMsg || lastMsg.id !== assistantMsg.id) return state;
                        let newContent: string;
                        const trimmed = lastMsg.content.trimStart();
                        if (trimmed.startsWith("[")) {
                            try {
                                const blocks = JSON.parse(lastMsg.content) as ContentBlock[];
                                const textBlock = blocks.find((b) => b.type === "text");
                                if (textBlock && textBlock.type === "text") {
                                    const updated = blocks.map((b) =>
                                        b.type === "text"
                                            ? { ...b, text: b.text + event.payload.content }
                                            : b
                                    );
                                    newContent = JSON.stringify(updated);
                                } else {
                                    newContent = lastMsg.content + event.payload.content;
                                }
                            } catch {
                                newContent = lastMsg.content + event.payload.content;
                            }
                        } else {
                            newContent = lastMsg.content + event.payload.content;
                        }
                        return {
                            chats: state.chats.map((c) =>
                                c.id === activeChatId
                                    ? {
                                          ...c,
                                          messages: c.messages.map((msg) =>
                                              msg.id === assistantMsg.id
                                                  ? { ...msg, content: newContent }
                                                  : msg
                                          ),
                                      }
                                    : c
                            ),
                        };
                    });
                })
            );

            unlisteners.push(
                await listen<StreamImagePayload>("chat-stream-image", (event) => {
                    const payload = event.payload;
                    if (payload.messageId !== assistantMsg.id) return;
                    streamImagePathsEdit.push({ path: payload.path, index: payload.index });
                    const imageBlock: ContentBlock = {
                        type: "image",
                        path: payload.path,
                        name: `image_${payload.index}.png`,
                    };
                    set((state) => {
                        const chat = state.chats.find((c) => c.id === activeChatId);
                        if (!chat) return state;
                        const msg = chat.messages.find((m) => m.id === assistantMsg.id);
                        if (!msg) return state;
                        let blocks: ContentBlock[];
                        const trimmed = msg.content.trimStart();
                        if (trimmed.startsWith("[")) {
                            try {
                                blocks = JSON.parse(msg.content) as ContentBlock[];
                            } catch {
                                blocks = [{ type: "text", text: msg.content }];
                            }
                            blocks = [...blocks, imageBlock];
                        } else {
                            blocks = [{ type: "text", text: msg.content }, imageBlock];
                        }
                        return {
                            chats: state.chats.map((c) =>
                                c.id === activeChatId
                                    ? {
                                          ...c,
                                          messages: c.messages.map((m) =>
                                              m.id === assistantMsg.id
                                                  ? { ...m, content: JSON.stringify(blocks) }
                                                  : m
                                          ),
                                      }
                                    : c
                            ),
                        };
                    });
                })
            );

            unlisteners.push(
                await listen<StreamUsagePayload>("chat-stream-usage", (event) => {
                    const { models, chats } = get();
                    const payload = event.payload;
                    const currentChat = chats.find((c) => c.id === activeChatId);
                    const modelId = currentChat?.model ?? "";
                    const modelInfo = modelId ? models.find((m) => m.id === modelId) : null;
                    let cost = 0;
                    if (modelInfo?.pricing) {
                        cost =
                            payload.prompt_tokens * parseFloat(modelInfo.pricing.prompt) +
                            payload.completion_tokens * parseFloat(modelInfo.pricing.completion);
                    }
                    invoke("update_message_usage", {
                        id: assistantMsg.id,
                        promptTokens: payload.prompt_tokens,
                        completionTokens: payload.completion_tokens,
                        cost,
                    }).catch(console.error);
                    set((state) => ({
                        chats: state.chats.map((c) =>
                            c.id === activeChatId
                                ? {
                                      ...c,
                                      messages: c.messages.map((m) =>
                                          m.id === assistantMsg.id
                                              ? {
                                                    ...m,
                                                    promptTokens: payload.prompt_tokens,
                                                    completionTokens: payload.completion_tokens,
                                                    cost,
                                                }
                                              : m
                                      ),
                                  }
                                : c
                        ),
                    }));
                })
            );

            unlisteners.push(
                await listen<StreamDonePayload>("chat-stream-done", (event) => {
                    const fullContentEdit = event.payload.full_content;
                    let contentToSaveEdit: string;
                    if (streamImagePathsEdit.length > 0) {
                        const sorted = [...streamImagePathsEdit].sort((a, b) => a.index - b.index);
                        const blocks: ContentBlock[] = [
                            { type: "text", text: fullContentEdit },
                            ...sorted.map((p) => ({
                                type: "image" as const,
                                path: p.path,
                                name: `image_${p.index}.png`,
                            })),
                        ];
                        contentToSaveEdit = JSON.stringify(blocks);
                    } else {
                        contentToSaveEdit = fullContentEdit;
                    }
                    invoke("update_message_content", {
                        id: assistantMsg.id,
                        content: contentToSaveEdit,
                    }).catch(console.error);
                    set((state) => ({
                        isStreaming: false,
                        isStopping: false,
                        chats: state.chats.map((c) =>
                            c.id === activeChatId
                                ? {
                                      ...c,
                                      messages: c.messages.map((m) =>
                                          m.id === assistantMsg.id
                                              ? { ...m, content: contentToSaveEdit }
                                              : m
                                      ),
                                  }
                                : c
                        ),
                    }));
                    unlisteners.forEach((fn) => fn());
                    get().loadBalance();
                    invoke("index_message", { messageId }).catch(console.error);
                    invoke("index_message", { messageId: assistantMsg.id }).catch(console.error);
                })
            );

            unlisteners.push(
                await listen<StreamErrorPayload>("chat-stream-error", (event) => {
                    notify.error(event.payload.error);
                    set({ isStreaming: false, isStopping: false });
                    unlisteners.forEach((fn) => fn());
                })
            );

            await invoke("send_message", {
                chatId: activeChatId,
                baseUrl: resolved.baseUrl,
                apiKey: resolved.apiKey,
                model: chat.model ?? "",
                messages: apiMessages,
                temperature: settings.temperature,
                maxTokens: settings.max_tokens,
                topP: settings.topP ?? null,
                topK: settings.topK ?? null,
                frequencyPenalty: settings.frequencyPenalty ?? null,
                presencePenalty: settings.presencePenalty ?? null,
                assistantMessageId: assistantMsg.id,
            });
        } catch (e) {
            set({ isStreaming: false });
            notify.error(String(e));
        }
    },

    stopGeneration: async () => {
        set({ isStopping: true });
        try {
            await invoke("stop_generation");
        } catch (e) {
            console.error("Failed to stop generation:", e);
            set({ isStopping: false });
        }
    },

    loadSettings: async () => {
        try {
            const loaded = await invoke<AppSettings>("load_settings");
            const settings: AppSettings = {
                ...loaded,
                ollamaUrl: loaded.ollamaUrl ?? "http://localhost:11434/v1",
                openrouterEnabledModels: Array.isArray(loaded.openrouterEnabledModels) ? loaded.openrouterEnabledModels : [],
                ollamaEnabledModels: Array.isArray(loaded.ollamaEnabledModels) ? loaded.ollamaEnabledModels : [],
                customProviderEnabledModels: loaded.customProviderEnabledModels && typeof loaded.customProviderEnabledModels === "object"
                    ? loaded.customProviderEnabledModels
                    : {},
                language: loaded.language ?? "",
                sendByEnter: loaded.sendByEnter ?? true,
            };
            set({ settings });
            const lang = settings.language?.trim() || (navigator.language.startsWith("ru") ? "ru" : "en");
            i18n.changeLanguage(lang);
        } catch (e) {
            notify.error(i18n.t("notifications.settingsLoadError"));
        }
    },

    saveSettings: async (settings: AppSettings) => {
        try {
            await invoke("save_settings", { settings });
            set({ settings });
            notify.success(i18n.t("notifications.settingsSaved"));
        } catch (e) {
            notify.error(String(e));
        }
    },
}));

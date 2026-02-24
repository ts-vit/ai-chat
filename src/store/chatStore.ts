import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "../utils/notify";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
    Chat,
    Message,
    Preset,
    Category,
    Snippet,
    AppSettings,
    ModelInfo,
    BalanceInfo,
    StreamPayload,
    StreamDonePayload,
    StreamErrorPayload,
    StreamUsagePayload,
} from "../types";

interface DbChatResponse {
    id: string;
    title: string;
    createdAt: number;
    updatedAt: number;
    systemPrompt?: string;
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
    };
}

interface ChatState {
    chats: Chat[];
    activeChatId: string | null;
    settings: AppSettings;
    presets: Preset[];
    categories: Category[];
    snippets: Snippet[];
    insertSnippetText: string | null;
    isStreaming: boolean;
    isStopping: boolean;
    balance: BalanceInfo | null;

    currentView: "chat" | "settings" | "snippets";
    setView: (view: "chat" | "settings" | "snippets") => void;
    models: ModelInfo[];
    modelsLoading: boolean;
    modelsError: string | null;
    loadModels: (force?: boolean) => Promise<void>;
    loadBalance: () => Promise<void>;
    loadPresets: () => Promise<void>;
    createPreset: (name: string, content: string, isDefault: boolean) => Promise<void>;
    updatePreset: (id: string, name: string, content: string, isDefault: boolean) => Promise<void>;
    deletePreset: (id: string) => Promise<void>;
    setChatSystemPrompt: (chatId: string, systemPrompt: string) => Promise<void>;

    loadCategories: () => Promise<void>;
    createCategory: (name: string) => Promise<void>;
    updateCategory: (id: string, name: string) => Promise<void>;
    deleteCategory: (id: string) => Promise<void>;
    loadSnippets: () => Promise<void>;
    createSnippet: (name: string, content: string, categoryId: string) => Promise<void>;
    updateSnippet: (id: string, name: string, content: string, categoryId: string) => Promise<void>;
    deleteSnippet: (id: string) => Promise<void>;
    setInsertSnippetText: (text: string | null) => void;

    createChat: () => Promise<void>;
    deleteChat: (id: string) => Promise<void>;
    setActiveChat: (id: string) => Promise<void>;
    loadChats: () => Promise<void>;

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
    categories: [],
    snippets: [],
    insertSnippetText: null,
    settings: {
        api_key: "",
        management_key: "",
        model: "anthropic/claude-sonnet-4-20250514",
        temperature: 0.7,
        max_tokens: 4096,
        font_size: 14,
    },
    isStreaming: false,
    isStopping: false,
    balance: null,

    currentView: "chat",
    setView: (view) => set({ currentView: view }),
    models: [],
    modelsLoading: false,
    modelsError: null,
    loadModels: async (force) => {
        const { models } = get();
        if (!force && models.length > 0) return;
        set({ modelsLoading: true, modelsError: null, ...(force ? { models: [] } : {}) });
        try {
            const response = await invoke<{ data: unknown }>("get_models");
            const raw = response?.data;
            if (!Array.isArray(raw)) {
                set({ models: [], modelsLoading: false, modelsError: "Неверный формат ответа" });
                return;
            }
            const list: ModelInfo[] = raw
                .filter(
                    (m: unknown): m is ModelInfo =>
                        typeof m === "object" &&
                        m !== null &&
                        typeof (m as ModelInfo).id === "string" &&
                        typeof (m as ModelInfo).name === "string" &&
                        typeof (m as ModelInfo).context_length === "number" &&
                        typeof (m as ModelInfo).pricing === "object" &&
                        (m as ModelInfo).pricing !== null &&
                        typeof (m as ModelInfo).pricing.prompt === "string" &&
                        typeof (m as ModelInfo).pricing.completion === "string"
                )
                .map((m) => ({
                    id: m.id,
                    name: m.name,
                    pricing: { prompt: m.pricing.prompt, completion: m.pricing.completion },
                    context_length: m.context_length,
                }));
            set({ models: list, modelsLoading: false, modelsError: null });
        } catch (e) {
            set({ modelsLoading: false, modelsError: String(e) });
            notify.error("Не удалось загрузить список моделей");
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
            notify.success("Пресет создан");
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
            notify.success("Пресет обновлён");
        } catch (e) {
            notify.error(String(e));
        }
    },

    deletePreset: async (id) => {
        try {
            await invoke("delete_preset", { id });
            set((state) => ({ presets: state.presets.filter((p) => p.id !== id) }));
            notify.success("Пресет удалён");
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
            notify.success("Категория создана");
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
            notify.success("Категория обновлена");
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteCategory: async (id) => {
        try {
            await invoke("delete_category", { id });
            set((state) => ({ categories: state.categories.filter((c) => c.id !== id) }));
            notify.success("Категория удалена");
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
            notify.success("Шаблон создан");
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
            notify.success("Шаблон обновлён");
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteSnippet: async (id) => {
        try {
            await invoke("delete_snippet", { id });
            set((state) => ({ snippets: state.snippets.filter((s) => s.id !== id) }));
            notify.success("Шаблон удалён");
        } catch (e) {
            notify.error(String(e));
        }
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

    createChat: async () => {
        try {
            const { presets } = get();
            const defaultPreset = presets.find((p) => p.isDefault);
            const systemPrompt = defaultPreset?.content ?? "";

            const created = await invoke<DbChatResponse>("create_chat", {
                title: "Новый чат",
                systemPrompt,
            });
            const newChat: Chat = {
                id: created.id,
                title: created.title,
                messages: [],
                createdAt: created.createdAt,
                updatedAt: created.updatedAt,
                systemPrompt: systemPrompt || undefined,
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
            notify.info("Чат удалён");
        } catch (e) {
            notify.error(String(e));
        }
    },

    setActiveChat: async (id: string) => {
        set({ activeChatId: id });
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
            }));
            set({ chats });
        } catch (e) {
            console.error("Failed to load chats:", e);
        }
    },

    sendMessage: async (content: string) => {
        const { settings, activeChatId, chats } = get();

        if (!settings.api_key) {
            notify.warning("API-ключ не задан. Откройте настройки.");
            return;
        }

        if (!activeChatId) return;

        const chat = chats.find((c) => c.id === activeChatId);
        if (!chat) return;

        const now = Date.now();

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
            const assistantMsg = await invoke<DbMessageResponse>("save_message", {
                chatId: activeChatId,
                role: "assistant",
                content: "",
                parentId: userMsg.id,
                timestamp: now + 1,
                model: settings.model,
                promptTokens: 0,
                completionTokens: 0,
                cost: 0.0,
            });
            const userMessage: Message = dbMessageToMessage(userMsg);
            const assistantMessage: Message = {
                ...dbMessageToMessage(assistantMsg),
                content: "",
                model: settings.model,
            };
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === activeChatId
                        ? {
                              ...c,
                              messages: [...c.messages, userMessage, assistantMessage],
                              title:
                                  c.messages.length === 0
                                      ? content.slice(0, 30) +
                                        (content.length > 30 ? "..." : "")
                                      : c.title,
                          }
                        : c
                ),
                isStreaming: true,
            }));

            if (chat.messages.length === 0) {
                const title =
                    content.slice(0, 30) + (content.length > 30 ? "..." : "");
                await invoke("update_chat_title", {
                    id: activeChatId,
                    title,
                });
            }

            const unlisteners: UnlistenFn[] = [];

            unlisteners.push(
                await listen<StreamPayload>("chat-stream", (event) => {
                    set((state) => ({
                        chats: state.chats.map((c) =>
                            c.id === activeChatId
                                ? {
                                      ...c,
                                      messages: c.messages.map((msg, idx) =>
                                          idx === c.messages.length - 1
                                              ? {
                                                    ...msg,
                                                    content:
                                                        msg.content +
                                                        event.payload.content,
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
                await listen<StreamUsagePayload>("chat-stream-usage", (event) => {
                    const { models, settings } = get();
                    const payload = event.payload;
                    const modelInfo = models.find((m) => m.id === settings.model);
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
                    invoke("update_message_content", {
                        id: assistantMsg.id,
                        content: event.payload.full_content,
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
                                              ? {
                                                    ...msg,
                                                    content:
                                                        event.payload.full_content,
                                                }
                                              : msg
                                      ),
                                  }
                                : c
                        ),
                    }));
                    unlisteners.forEach((fn) => fn());
                    get().loadBalance();
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
                .map((m) => ({ role: m.role, content: m.content }));
            const apiMessages = [
                ...(chat.systemPrompt?.trim()
                    ? [{ role: "system" as const, content: chat.systemPrompt }]
                    : []),
                ...history,
                { role: "user" as const, content },
            ];
            await invoke("send_message", {
                apiKey: settings.api_key,
                model: settings.model,
                messages: apiMessages,
                temperature: settings.temperature,
                maxTokens: settings.max_tokens,
            });
        } catch (e) {
            set({ isStreaming: false });
            notify.error(String(e));
        }
    },

    editAndResend: async (messageId: string, newContent: string) => {
        const { settings, activeChatId, chats, isStreaming } = get();

        if (isStreaming) return;
        if (!settings.api_key) {
            notify.warning("API-ключ не задан. Откройте настройки.");
            return;
        }
        if (!activeChatId) return;

        const chat = chats.find((c) => c.id === activeChatId);
        if (!chat) return;

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
                .map((m) => ({ role: m.role, content: m.content }));
            const apiMessages = [
                ...(chatAfter.systemPrompt?.trim()
                    ? [{ role: "system" as const, content: chatAfter.systemPrompt }]
                    : []),
                ...history,
            ];

            const assistantMsg = await invoke<DbMessageResponse>("save_message", {
                chatId: activeChatId,
                role: "assistant",
                content: "",
                parentId: messageId,
                timestamp: msg.timestamp + 1,
                model: settings.model,
                promptTokens: 0,
                completionTokens: 0,
                cost: 0.0,
            });

            const assistantMessage: Message = {
                ...dbMessageToMessage(assistantMsg),
                content: "",
                model: settings.model,
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

            unlisteners.push(
                await listen<StreamPayload>("chat-stream", (event) => {
                    set((state) => ({
                        chats: state.chats.map((c) =>
                            c.id === activeChatId
                                ? {
                                      ...c,
                                      messages: c.messages.map((m, i) =>
                                          i === c.messages.length - 1
                                              ? {
                                                    ...m,
                                                    content: m.content + event.payload.content,
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
                await listen<StreamUsagePayload>("chat-stream-usage", (event) => {
                    const { models, settings: s } = get();
                    const payload = event.payload;
                    const modelInfo = models.find((m) => m.id === s.model);
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
                    invoke("update_message_content", {
                        id: assistantMsg.id,
                        content: event.payload.full_content,
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
                                              ? { ...m, content: event.payload.full_content }
                                              : m
                                      ),
                                  }
                                : c
                        ),
                    }));
                    unlisteners.forEach((fn) => fn());
                    get().loadBalance();
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
                apiKey: settings.api_key,
                model: settings.model,
                messages: apiMessages,
                temperature: settings.temperature,
                maxTokens: settings.max_tokens,
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
            const settings = await invoke<AppSettings>("load_settings");
            set({ settings });
        } catch (e) {
            notify.error("Не удалось загрузить настройки");
        }
    },

    saveSettings: async (settings: AppSettings) => {
        try {
            await invoke("save_settings", { settings });
            set({ settings });
            notify.success("Настройки сохранены");
        } catch (e) {
            notify.error(String(e));
        }
    },
}));

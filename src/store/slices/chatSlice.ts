import { invoke } from "@tauri-apps/api/core";
import i18n from "../../i18n";
import { notify } from "../../utils/notify";
import { resolveInjections } from "../../utils/injections";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
    Attachment,
    Chat,
    ContentBlock,
    Folder,
    MessageWithSiblings,
    StreamPayload,
    StreamDonePayload,
    StreamErrorPayload,
    StreamUsagePayload,
    StreamImagePayload,
    ToolCallEvent,
    ToolResultEvent,
    ToolCallInfo,
    WebSource,
    StreamRetryPayload,
    ProjectSummary,
} from "../../types";
import type { ChatState } from "../chatStore";
import {
    type DbChatResponse,
    type DbMessageResponse,
    type DbMessageWithSiblingsResponse,
    resolveProvider,
    extractTextContent,
    stripMarkdownForTts,
    dbMessageToMessage,
    dbMessageToMessageWithSiblings,
    toMessageWithSiblings,
    _initModeState,
    _initActiveMode,
    parseRagData,
} from "../helpers";

export interface ChatSlice {
    // State
    chats: Chat[];
    activeChatId: string | null;
    folders: Folder[];
    projects: ProjectSummary[];
    activeProjectId: string | null;
    isStreaming: boolean;
    isStopping: boolean;
    webSearchEnabled: boolean;
    isSearching: boolean;
    draftAttachments: Attachment[];
    activeToolCalls: ToolCallInfo[];
    playingMessageId: string | null;
    scrollTargetId: string | null;

    // Actions — Attachments
    addAttachment: (attachment: Attachment) => void;
    removeAttachment: (id: string) => void;
    clearAttachments: () => void;

    // Actions — Chat CRUD
    setScrollTargetId: (id: string | null) => void;
    setChatSystemPrompt: (chatId: string, systemPrompt: string) => Promise<void>;
    updateChatModel: (chatId: string, model: string, isImageModel?: boolean) => Promise<void>;
    updateChatParams: (
        chatId: string,
        params: {
            temperature?: number | null;
            maxTokens?: number | null;
            topP?: number | null;
            topK?: number | null;
            frequencyPenalty?: number | null;
            presencePenalty?: number | null;
        }
    ) => Promise<void>;
    updateImageConfig: (
        chatId: string,
        config: {
            imageSize?: string | null;
            imageQuality?: string | null;
            imageStyle?: string | null;
            imageN?: number | null;
        }
    ) => Promise<void>;
    updateChatNegativePrompt: (chatId: string, negativePrompt: string | null) => Promise<void>;
    toggleWebSearch: () => void;
    createChat: (
        providerId?: string,
        model?: string,
        isImageModel?: boolean,
        params?: {
            temperature?: number | null;
            maxTokens?: number | null;
            topP?: number | null;
            topK?: number | null;
            frequencyPenalty?: number | null;
            presencePenalty?: number | null;
        },
        projectId?: string | null
    ) => Promise<void>;
    deleteChat: (id: string) => Promise<void>;
    setActiveChat: (id: string) => Promise<void>;
    loadChats: () => Promise<void>;

    // Actions — Folders
    loadFolders: () => Promise<void>;
    createFolder: (name: string, color: string | null) => Promise<void>;
    updateFolder: (id: string, name: string, color: string | null) => Promise<void>;
    deleteFolder: (id: string) => Promise<void>;
    reorderFolders: (ids: string[]) => Promise<void>;
    moveChatToFolder: (chatId: string, folderId: string | null) => Promise<void>;

    // Actions — Projects
    loadProjects: () => Promise<void>;
    createProject: (name: string, goal: string) => Promise<void>;
    updateProject: (id: string, updates: { name?: string; goal?: string; status?: string }) => Promise<void>;
    deleteProject: (id: string) => Promise<void>;
    archiveProject: (id: string) => Promise<void>;
    assignChatToProject: (chatId: string, projectId: string) => Promise<void>;
    removeChatFromProject: (chatId: string) => Promise<void>;
    setActiveProjectId: (id: string | null) => void;

    // Actions — Messaging
    sendMessage: (content: string) => Promise<void>;
    editAndResend: (messageId: string, newContent: string) => Promise<void>;
    switchBranch: (messageId: string) => Promise<void>;
    stopGeneration: () => Promise<void>;

    // Actions — TTS
    setTtsPlayingMessageId: (id: string | null) => void;
    speakMessage: (messageId: string, content: string) => Promise<void>;
    stopTts: () => Promise<void>;
}

type Set = (partial: Partial<ChatState> | ((state: ChatState) => Partial<ChatState>)) => void;
type Get = () => ChatState;

export const createChatSlice = (set: Set, get: Get): ChatSlice => ({
    // ── State ──────────────────────────────────────────────
    chats: [],
    activeChatId: _initModeState[_initActiveMode]?.activeChatId ?? null,
    folders: [],
    projects: [],
    activeProjectId: null,
    isStreaming: false,
    isStopping: false,
    webSearchEnabled: true,
    isSearching: false,
    draftAttachments: [],
    activeToolCalls: [],
    playingMessageId: null,
    scrollTargetId: null,

    // ── Attachments ───────────────────────────────────────

    addAttachment: (attachment) =>
        set((state) => ({ draftAttachments: [...state.draftAttachments, attachment] })),
    removeAttachment: (id) =>
        set((state) => ({ draftAttachments: state.draftAttachments.filter((a) => a.id !== id) })),
    clearAttachments: () => set({ draftAttachments: [] }),

    // ── Misc ──────────────────────────────────────────────

    setScrollTargetId: (id) => set({ scrollTargetId: id }),

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

    toggleWebSearch: () => set((state) => ({ webSearchEnabled: !state.webSearchEnabled })),

    // ── Chat CRUD ─────────────────────────────────────────

    createChat: async (
        providerId = "openrouter",
        model = "",
        isImageModel = false,
        params,
        projectId?: string | null
    ) => {
        try {
            const { presets } = get();
            const defaultPreset = presets.find((p) => p.isDefault);
            const systemPrompt = defaultPreset?.content ?? "";

            const payload: Record<string, unknown> = {
                title: i18n.t("chat.newChatTitle"),
                systemPrompt,
                providerId,
                model,
                isImageModel,
                mode: get().activeMode,
                projectId: projectId ?? undefined,
            };
            if (params != null && typeof params.temperature === "number") payload.temperature = params.temperature;
            if (params != null && typeof params.maxTokens === "number") payload.maxTokens = params.maxTokens;
            if (params != null && typeof params.topP === "number") payload.topP = params.topP;
            if (params != null && typeof params.topK === "number") payload.topK = params.topK;
            if (params != null && typeof params.frequencyPenalty === "number") payload.frequencyPenalty = params.frequencyPenalty;
            if (params != null && typeof params.presencePenalty === "number") payload.presencePenalty = params.presencePenalty;

            const created = await invoke<DbChatResponse>("create_chat", payload);
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
                projectId: created.projectId ?? null,
                temperature: created.temperature ?? undefined,
                maxTokens: created.maxTokens ?? undefined,
                topP: created.topP ?? undefined,
                topK: created.topK ?? undefined,
                frequencyPenalty: created.frequencyPenalty ?? undefined,
                presencePenalty: created.presencePenalty ?? undefined,
                imageSize: created.imageSize ?? undefined,
                imageQuality: created.imageQuality ?? undefined,
                imageStyle: created.imageStyle ?? undefined,
                imageN: created.imageN ?? undefined,
                negativePrompt: created.negativePrompt ?? undefined,
                mode: created.mode ?? get().activeMode,
            };
            const { activeMode, modeState } = get();
            const newModeState = {
                ...modeState,
                [activeMode]: { ...modeState[activeMode], activeChatId: newChat.id },
            };
            set((state) => ({
                chats: [newChat, ...state.chats],
                activeChatId: newChat.id,
                modeState: newModeState,
            }));
            localStorage.setItem('uni-mode-state', JSON.stringify(newModeState));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteChat: async (id: string) => {
        try {
            await invoke("delete_search_index", { chatId: id }).catch(console.error);
            await invoke("delete_chat", { id });

            const { modeState, activeMode, chats } = get();
            const deletedChat = chats.find((c) => c.id === id);
            const chatMode = deletedChat?.mode ?? 'chat';
            const remaining = chats.filter((c) => c.id !== id);

            if (modeState[chatMode]?.activeChatId === id) {
                const sameModeChats = remaining.filter((c) => (c.mode ?? 'chat') === chatMode);
                const nextId = sameModeChats[0]?.id ?? null;
                const newModeState = {
                    ...modeState,
                    [chatMode]: { ...modeState[chatMode], activeChatId: nextId },
                };
                const updates: Partial<ChatState> = { chats: remaining, modeState: newModeState };
                if (chatMode === activeMode) {
                    updates.activeChatId = nextId;
                }
                set(updates as ChatState);
                localStorage.setItem('uni-mode-state', JSON.stringify(newModeState));
            } else {
                set({ chats: remaining });
            }

            notify.info(i18n.t("notifications.chatDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    setActiveChat: async (id: string) => {
        const { activeMode, modeState } = get();
        const newModeState = {
            ...modeState,
            [activeMode]: { ...modeState[activeMode], activeChatId: id },
        };
        set({ activeChatId: id, modeState: newModeState, draftAttachments: [] });
        localStorage.setItem('uni-mode-state', JSON.stringify(newModeState));
        const { chats } = get();
        const chat = chats.find((c) => c.id === id);
        if (chat && chat.messages.length === 0) {
            try {
                const rows = await invoke<DbMessageWithSiblingsResponse[]>("get_messages_branched", {
                    chatId: id,
                });
                const messages = rows.map(dbMessageToMessageWithSiblings);
                set((state) => ({
                    chats: state.chats.map((c) =>
                        c.id === id ? { ...c, messages } : c
                    ),
                }));
            } catch (e) {
                console.error("Failed to load messages:", e);
            }
        }
        // Load plan and workspace for assistant mode chats
        if (activeMode === "assistant") {
            get().loadPlanForChat(id);
            get().loadWorkspaceArtifacts(id);
        }
        // Load attached KB
        get().loadChatKb(id);
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
                projectId: c.projectId ?? null,
                temperature: c.temperature ?? undefined,
                maxTokens: c.maxTokens ?? undefined,
                topP: c.topP ?? undefined,
                topK: c.topK ?? undefined,
                frequencyPenalty: c.frequencyPenalty ?? undefined,
                presencePenalty: c.presencePenalty ?? undefined,
                imageSize: c.imageSize ?? undefined,
                imageQuality: c.imageQuality ?? undefined,
                imageStyle: c.imageStyle ?? undefined,
                imageN: c.imageN ?? undefined,
                negativePrompt: c.negativePrompt ?? undefined,
                mode: c.mode ?? "chat",
            }));
            set({ chats });
        } catch (e) {
            console.error("Failed to load chats:", e);
        }
    },

    // ── Folders ───────────────────────────────────────────

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
            const folder = await invoke<Folder>("create_folder", { name, color, mode: get().activeMode });
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

    // ── Projects ──────────────────────────────────────────

    loadProjects: async () => {
        try {
            const list = await invoke<ProjectSummary[]>("list_projects", { statusFilter: null });
            set({ projects: list ?? [] });
        } catch (e) {
            console.error("Failed to load projects:", e);
        }
    },

    createProject: async (name: string, goal: string) => {
        try {
            const project = await invoke<ProjectSummary>("create_project", { name, goal });
            set((state) => ({ projects: [{ ...project, chatCount: 0, artifactCount: 0, memoryCount: 0, planCount: 0 }, ...state.projects] }));
            notify.success(i18n.t("notifications.projectCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateProject: async (id: string, updates: { name?: string; goal?: string; status?: string }) => {
        try {
            await invoke("update_project", { id, ...updates });
            set((state) => ({
                projects: state.projects.map((p) =>
                    p.id === id ? { ...p, ...updates as Partial<ProjectSummary>, updatedAt: Math.floor(Date.now() / 1000) } : p
                ),
            }));
            notify.success(i18n.t("notifications.projectUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteProject: async (id: string) => {
        try {
            await invoke("delete_project", { id });
            set((state) => ({
                projects: state.projects.filter((p) => p.id !== id),
                chats: state.chats.map((c) =>
                    c.projectId === id ? { ...c, projectId: null } : c
                ),
                activeProjectId: state.activeProjectId === id ? null : state.activeProjectId,
            }));
            notify.success(i18n.t("notifications.projectDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    archiveProject: async (id: string) => {
        try {
            await invoke("archive_project", { id });
            set((state) => ({
                projects: state.projects.map((p) =>
                    p.id === id ? { ...p, status: "archived" as const, updatedAt: Math.floor(Date.now() / 1000) } : p
                ),
            }));
            notify.success(i18n.t("notifications.projectArchived"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    assignChatToProject: async (chatId: string, projectId: string) => {
        try {
            await invoke("assign_chat_to_project", { chatId, projectId });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, projectId } : c
                ),
            }));
            get().loadProjects();
        } catch (e) {
            notify.error(String(e));
        }
    },

    removeChatFromProject: async (chatId: string) => {
        try {
            await invoke("remove_chat_from_project", { chatId });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, projectId: null } : c
                ),
            }));
            get().loadProjects();
        } catch (e) {
            notify.error(String(e));
        }
    },

    setActiveProjectId: (id: string | null) => set({ activeProjectId: id }),

    // ── sendMessage ───────────────────────────────────────

    sendMessage: async (content: string) => {
        const { settings, activeChatId, chats, customProviders, webSearchEnabled } = get();

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
            content = await resolveInjections(content);

            const lastMsg = chat.messages[chat.messages.length - 1];
            const userMsg = await invoke<DbMessageResponse>("save_message", {
                chatId: activeChatId,
                role: "user",
                content,
                parentId: lastMsg?.id ?? null,
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
            const userMessage: MessageWithSiblings = toMessageWithSiblings(dbMessageToMessage(userMsg));
            const assistantMessage: MessageWithSiblings = {
                ...toMessageWithSiblings(dbMessageToMessage(assistantMsg)),
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

            let searchContext = "";
            let pendingSourcesList: WebSource[] = [];
            if (webSearchEnabled) {
                set({ isSearching: true });
                try {
                    interface WebSearchResult {
                        title: string;
                        url: string;
                        snippet: string;
                        content?: string;
                    }
                    const results = await invoke<WebSearchResult[]>("web_search", {
                        query: content,
                        numResults: 5,
                        provider: settings.webSearchProvider ?? "duckduckgo",
                        tavilyApiKey: settings.tavilyApiKey ?? null,
                        braveApiKey: settings.braveApiKey ?? null,
                    });
                    if (results && results.length > 0) {
                        pendingSourcesList = results.map((r) => ({
                            title: r.title,
                            url: r.url,
                            snippet: r.snippet,
                        }));
                        let ctx = `[Web Search Results]\n\n`;
                        let wordCount = 0;
                        const wordLimit = Math.floor(4000 / 1.3);
                        for (let idx = 0; idx < results.length; idx++) {
                            const r = results[idx];
                            const header = `[${idx + 1}] ${r.title} (${r.url})\n`;
                            const body = r.content?.trim() || r.snippet;
                            const headerWords = header.split(/\s+/).length;
                            const bodyWords = body.split(/\s+/);
                            if (wordCount + headerWords >= wordLimit) break;
                            ctx += header;
                            const remaining = wordLimit - wordCount - headerWords;
                            if (bodyWords.length <= remaining) {
                                ctx += body;
                            } else {
                                ctx += bodyWords.slice(0, remaining).join(" ") + "...";
                            }
                            ctx += "\n\n";
                            wordCount += headerWords + Math.min(bodyWords.length, remaining);
                        }
                        ctx +=
                            "Answer the user's question using the search results above. When citing information, use numbered references like [1], [2], etc. corresponding to the source numbers above. Do not write \"Source 1\" — use only the bracket number format [1].";
                        searchContext = ctx;
                    }
                } catch (e) {
                    notify.warning(i18n.t("notifications.webSearch.error"));
                    console.error("Web search failed:", e);
                } finally {
                    set({ isSearching: false });
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
                    const provider = currentChat?.providerId ?? "openrouter";
                    invoke("update_message_usage", {
                        id: assistantMsg.id,
                        promptTokens: payload.prompt_tokens,
                        completionTokens: payload.completion_tokens,
                        cost,
                        chatId: activeChatId ?? undefined,
                        modelId: modelId || undefined,
                        provider: provider || undefined,
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
                await listen<ToolCallEvent>("chat-stream-tool-call", (event) => {
                    const p = event.payload;
                    set((state) => ({
                        activeToolCalls: [
                            ...state.activeToolCalls,
                            {
                                toolCallId: p.toolCallId,
                                serverId: p.serverId,
                                toolName: p.toolName,
                                arguments: p.arguments,
                                status: "calling",
                            },
                        ],
                    }));
                })
            );

            unlisteners.push(
                await listen<ToolResultEvent>("chat-stream-tool-result", (event) => {
                    const p = event.payload;
                    set((state) => ({
                        activeToolCalls: state.activeToolCalls.map((tc) =>
                            tc.toolCallId === p.toolCallId
                                ? {
                                      ...tc,
                                      result: p.result,
                                      isError: p.isError,
                                      status: p.isError ? "error" as const : "done" as const,
                                  }
                                : tc
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
                        content: contentToSave ?? "",
                    }).catch(console.error);
                    set((state) => ({
                        isStreaming: false,
                        isStopping: false,
                        activeToolCalls: [],
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
                    if (pendingSourcesList.length > 0) {
                        invoke("update_message_web_sources", {
                            messageId: assistantMsg.id,
                            webSources: JSON.stringify(pendingSourcesList),
                        }).catch(console.error);
                        set((state) => ({
                            chats: state.chats.map((c) =>
                                c.id === activeChatId
                                    ? {
                                          ...c,
                                          messages: c.messages.map((msg) =>
                                              msg.id === assistantMsg.id
                                                  ? { ...msg, webSources: pendingSourcesList }
                                                  : msg
                                          ),
                                      }
                                    : c
                            ),
                        }));
                    }
                    // Handle RAG sources from KB
                    if (event.payload.ragSources) {
                        const ragData = parseRagData(event.payload.ragSources);
                        if (ragData?.sources && ragData.sources.length > 0) {
                            set((state) => ({
                                chats: state.chats.map((c) =>
                                    c.id === activeChatId
                                        ? {
                                              ...c,
                                              messages: c.messages.map((msg) =>
                                                  msg.id === assistantMsg.id
                                                      ? { ...msg, ragSources: ragData.sources, ragTrace: ragData.trace }
                                                      : msg
                                              ),
                                          }
                                        : c
                                ),
                            }));
                        }
                    }
                })
            );

            unlisteners.push(
                await listen<StreamRetryPayload>("chat-stream-retry", (event) => {
                    const { attempt, maxAttempts } = event.payload;
                    notify.warning(
                        i18n.t("notifications.retrying", { attempt, maxAttempts }),
                        "chat-stream-retry"
                    );
                })
            );

            unlisteners.push(
                await listen<StreamErrorPayload>("chat-stream-error", (event) => {
                    notify.error(event.payload.error);
                    set({ isStreaming: false, isStopping: false, activeToolCalls: [] });
                    unlisteners.forEach((fn) => fn());
                })
            );

            const history = chat.messages
                .filter((m) => m.role !== "assistant" || m.content)
                .map((m) => ({ role: m.role, content: extractTextContent(m.content) }));
            const resolvedSystemPrompt = chat.systemPrompt?.trim()
                ? await resolveInjections(chat.systemPrompt)
                : undefined;
            const apiMessages = [
                ...(resolvedSystemPrompt
                    ? [{ role: "system" as const, content: resolvedSystemPrompt }]
                    : []),
                ...(searchContext
                    ? [{ role: "system" as const, content: searchContext }]
                    : []),
                ...history,
                { role: "user" as const, content },
            ];
            const { draftAttachments } = get();
            const currentModel = get().models.find(m => m.id === (chat.model ?? ""));
            const supportsToolUse = currentModel?.supportsToolUse ?? true;
            const invokePayload: Record<string, unknown> = {
                chatId: activeChatId,
                baseUrl: resolved.baseUrl,
                apiKey: resolved.apiKey,
                model: chat.model ?? "",
                messages: apiMessages,
                temperature: chat.temperature ?? settings.temperature,
                maxTokens: chat.maxTokens ?? settings.max_tokens,
                topP: chat.topP ?? settings.topP ?? null,
                topK: chat.topK ?? settings.topK ?? null,
                frequencyPenalty: chat.frequencyPenalty ?? settings.frequencyPenalty ?? null,
                presencePenalty: chat.presencePenalty ?? settings.presencePenalty ?? null,
                supportsToolUse,
                supportsImageGeneration: currentModel?.supportsImageGeneration ?? false,
                assistantMessageId: assistantMsg.id,
                imageSize: chat.imageSize ?? null,
                imageQuality: chat.imageQuality ?? null,
                imageStyle: chat.imageStyle ?? null,
                imageN: chat.imageN ?? null,
                negativePrompt: chat.negativePrompt ?? null,
                ragMode: get().ragMode !== "auto" ? get().ragMode : null,
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

    // ── editAndResend ─────────────────────────────────────

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
            newContent = await resolveInjections(newContent);

            const now = Math.floor(Date.now() / 1000);

            const newUserMsg = await invoke<DbMessageResponse>("save_message", {
                chatId: activeChatId,
                role: "user",
                content: newContent,
                parentId: msg.parentId ?? null,
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
                parentId: newUserMsg.id,
                timestamp: now + 1,
                model: chatModel,
                promptTokens: 0,
                completionTokens: 0,
                cost: 0.0,
            });

            const branchedRows = await invoke<DbMessageWithSiblingsResponse[]>("get_messages_branched", {
                chatId: activeChatId,
            });
            const branchedMessages = branchedRows.map(dbMessageToMessageWithSiblings);

            const history = branchedMessages
                .filter((m) => m.role !== "assistant" || m.content)
                .map((m) => ({ role: m.role, content: extractTextContent(m.content) }));
            const resolvedSystemPrompt = chat.systemPrompt?.trim()
                ? await resolveInjections(chat.systemPrompt)
                : undefined;
            const apiMessages = [
                ...(resolvedSystemPrompt
                    ? [{ role: "system" as const, content: resolvedSystemPrompt }]
                    : []),
                ...history,
            ];

            if (!apiMessages.some((m) => m.role === "user")) {
                notify.error(i18n.t("notifications.emptyMessageHistory"));
                return;
            }

            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === activeChatId ? { ...c, messages: branchedMessages } : c
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
                    const provider = currentChat?.providerId ?? "openrouter";
                    invoke("update_message_usage", {
                        id: assistantMsg.id,
                        promptTokens: payload.prompt_tokens,
                        completionTokens: payload.completion_tokens,
                        cost,
                        chatId: activeChatId ?? undefined,
                        modelId: modelId || undefined,
                        provider: provider || undefined,
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
                await listen<ToolCallEvent>("chat-stream-tool-call", (event) => {
                    const p = event.payload;
                    set((state) => ({
                        activeToolCalls: [
                            ...state.activeToolCalls,
                            {
                                toolCallId: p.toolCallId,
                                serverId: p.serverId,
                                toolName: p.toolName,
                                arguments: p.arguments,
                                status: "calling",
                            },
                        ],
                    }));
                })
            );

            unlisteners.push(
                await listen<ToolResultEvent>("chat-stream-tool-result", (event) => {
                    const p = event.payload;
                    set((state) => ({
                        activeToolCalls: state.activeToolCalls.map((tc) =>
                            tc.toolCallId === p.toolCallId
                                ? {
                                      ...tc,
                                      result: p.result,
                                      isError: p.isError,
                                      status: p.isError ? "error" as const : "done" as const,
                                  }
                                : tc
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
                        content: contentToSaveEdit ?? "",
                    }).catch(console.error);
                    set((state) => ({
                        isStreaming: false,
                        isStopping: false,
                        activeToolCalls: [],
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
                    invoke("index_message", { messageId: newUserMsg.id }).catch(console.error);
                    invoke("index_message", { messageId: assistantMsg.id }).catch(console.error);
                    // Handle RAG sources from KB
                    if (event.payload.ragSources) {
                        const ragData = parseRagData(event.payload.ragSources);
                        if (ragData?.sources && ragData.sources.length > 0) {
                            set((state) => ({
                                chats: state.chats.map((c) =>
                                    c.id === activeChatId
                                        ? {
                                              ...c,
                                              messages: c.messages.map((m) =>
                                                  m.id === assistantMsg.id
                                                      ? { ...m, ragSources: ragData.sources, ragTrace: ragData.trace }
                                                      : m
                                              ),
                                          }
                                        : c
                                ),
                            }));
                        }
                    }
                })
            );

            unlisteners.push(
                await listen<StreamRetryPayload>("chat-stream-retry", (event) => {
                    const { attempt, maxAttempts } = event.payload;
                    notify.warning(
                        i18n.t("notifications.retrying", { attempt, maxAttempts }),
                        "chat-stream-retry"
                    );
                })
            );

            unlisteners.push(
                await listen<StreamErrorPayload>("chat-stream-error", (event) => {
                    notify.error(event.payload.error);
                    set({ isStreaming: false, isStopping: false, activeToolCalls: [] });
                    unlisteners.forEach((fn) => fn());
                })
            );

            const currentModel = get().models.find(m => m.id === (chat.model ?? ""));
            const supportsToolUse = currentModel?.supportsToolUse ?? true;
            await invoke("send_message", {
                chatId: activeChatId,
                baseUrl: resolved.baseUrl,
                apiKey: resolved.apiKey,
                model: chat.model ?? "",
                messages: apiMessages,
                temperature: chat.temperature ?? settings.temperature,
                maxTokens: chat.maxTokens ?? settings.max_tokens,
                topP: chat.topP ?? settings.topP ?? null,
                topK: chat.topK ?? settings.topK ?? null,
                frequencyPenalty: chat.frequencyPenalty ?? settings.frequencyPenalty ?? null,
                presencePenalty: chat.presencePenalty ?? settings.presencePenalty ?? null,
                supportsToolUse,
                supportsImageGeneration: currentModel?.supportsImageGeneration ?? false,
                assistantMessageId: assistantMsg.id,
                imageSize: chat.imageSize ?? null,
                imageQuality: chat.imageQuality ?? null,
                imageStyle: chat.imageStyle ?? null,
                imageN: chat.imageN ?? null,
                negativePrompt: chat.negativePrompt ?? null,
                ragMode: get().ragMode !== "auto" ? get().ragMode : null,
            });
        } catch (e) {
            set({ isStreaming: false });
            notify.error(String(e));
        }
    },

    // ── switchBranch ──────────────────────────────────────

    switchBranch: async (messageId: string) => {
        const { activeChatId } = get();
        if (!activeChatId) return;
        try {
            const rows = await invoke<DbMessageWithSiblingsResponse[]>("switch_branch", {
                chatId: activeChatId,
                messageId,
            });
            const messages = rows.map(dbMessageToMessageWithSiblings);
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === activeChatId ? { ...c, messages } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Chat params ───────────────────────────────────────

    updateChatParams: async (chatId, params) => {
        try {
            await invoke("update_chat_params", { chatId, ...params });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, ...params } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateImageConfig: async (chatId, config) => {
        try {
            await invoke("update_chat_image_config", { chatId, ...config });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, ...config } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateChatNegativePrompt: async (chatId, negativePrompt) => {
        try {
            await invoke("update_chat_negative_prompt", { chatId, negativePrompt });
            set((state) => ({
                chats: state.chats.map((c) =>
                    c.id === chatId ? { ...c, negativePrompt } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Generation control ────────────────────────────────

    stopGeneration: async () => {
        set({ isStopping: true });
        try {
            await invoke("stop_generation");
        } catch (e) {
            console.error("Failed to stop generation:", e);
            set({ isStopping: false });
        }
    },

    // ── TTS ───────────────────────────────────────────────

    setTtsPlayingMessageId: (id) => set({ playingMessageId: id }),

    speakMessage: async (messageId, content) => {
        const { settings, playingMessageId } = get();
        if (playingMessageId === messageId) return;
        const plainText = stripMarkdownForTts(content);
        if (!plainText.trim()) return;
        const provider = settings.ttsProvider ?? "system";
        const voice = settings.ttsVoice || undefined;
        try {
            await invoke("tts_speak", {
                text: plainText,
                provider,
                voice: voice || null,
            });
            set({ playingMessageId: messageId });
        } catch (e) {
            notify.error(String(e));
        }
    },

    stopTts: async () => {
        try {
            await invoke("tts_stop");
            set({ playingMessageId: null });
        } catch (e) {
            console.error("Failed to stop TTS:", e);
            set({ playingMessageId: null });
        }
    },
});

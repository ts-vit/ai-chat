import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import i18n from "../../i18n";
import { notify } from "../../utils/notify";
import { resolveInjections } from "../../utils/injections";
import type {
    Comparison,
    ComparisonMessage,
    ComparisonStreamPayload,
    ComparisonStreamDonePayload,
    ComparisonStreamUsagePayload,
    ComparisonStreamErrorPayload,
    ComparisonStreamImagePayload,
    ComparisonStreamRetryPayload,
    ComparisonToolCallPayload,
    ComparisonToolResultPayload,
    ContentBlock,
    ModeSettingRow,
    ModeStateMap,
    ToolCallInfo,
} from "../../types";
import { resolveProvider, _initModeState, _initActiveMode } from "../helpers";
import type { ChatState } from "../chatStore";

export interface UiSlice {
    // State
    currentView: "chat" | "settings" | "snippets" | "search" | "compare" | "comparisons" | "promptLibrary" | "memory" | "skills" | "plans" | "projectDashboard" | "scheduler";
    activeMode: string;
    modeState: ModeStateMap;
    modeSettings: ModeSettingRow[];
    telegramStatus: { running: boolean; botUsername: string | null; authorizedUser: string | null };

    // Comparison state
    comparisons: Comparison[];
    activeComparisonId: string | null;
    comparisonMessages: ComparisonMessage[];
    compareStreamingLeft: boolean;
    compareStreamingRight: boolean;
    compareToolCallsLeft: ToolCallInfo[];
    compareToolCallsRight: ToolCallInfo[];

    // Actions — View & Navigation
    setView: (view: UiSlice["currentView"]) => void;
    openProjectDashboard: (projectId: string) => void;

    // Actions — Mode
    setActiveMode: (mode: string) => void;
    loadModeSettings: () => Promise<void>;
    updateModeSetting: (mode: string, enabled: boolean, sortOrder: number, config: string) => Promise<void>;

    // Actions — Comparisons
    loadComparisons: () => Promise<void>;
    setActiveComparison: (id: string) => Promise<void>;
    createComparison: (
        leftProviderId: string,
        leftModel: string,
        rightProviderId: string,
        rightModel: string,
        leftSystemPrompt?: string,
        rightSystemPrompt?: string,
    ) => Promise<void>;
    deleteComparison: (id: string) => Promise<void>;
    updateComparisonTitle: (id: string, title: string) => Promise<void>;
    sendComparisonMessage: (content: string) => Promise<void>;
    stopComparisonGeneration: () => Promise<void>;

    // Actions — Telegram
    startTelegramBot: () => Promise<void>;
    stopTelegramBot: () => Promise<void>;
    getTelegramStatus: () => Promise<void>;
    authorizeTelegramUser: (userId: number, firstName: string, lastName: string | null, username: string | null) => Promise<void>;
    revokeTelegramUser: () => Promise<void>;
    validateTelegramToken: (token: string) => Promise<{ username: string; firstName: string }>;
}

type Set = (partial: Partial<ChatState> | ((state: ChatState) => Partial<ChatState>)) => void;
type Get = () => ChatState;

export const createUiSlice = (set: Set, get: Get): UiSlice => ({
    // ── State ──────────────────────────────────────────────

    currentView: "chat",
    activeMode: _initActiveMode,
    modeState: _initModeState,
    modeSettings: [],
    telegramStatus: { running: false, botUsername: null, authorizedUser: null },

    comparisons: [],
    activeComparisonId: _initModeState[_initActiveMode]?.activeComparisonId ?? null,
    comparisonMessages: [],
    compareStreamingLeft: false,
    compareStreamingRight: false,
    compareToolCallsLeft: [],
    compareToolCallsRight: [],

    // ── View & Navigation ─────────────────────────────────

    setView: (view) => set({ currentView: view }),

    openProjectDashboard: (projectId: string) => set({ activeProjectId: projectId, currentView: "projectDashboard" }),

    // ── Mode ──────────────────────────────────────────────

    setActiveMode: (mode) => {
        const prev = get().activeMode;
        if (prev === mode) return;

        const { modeState } = get();
        const ws = modeState[mode];
        const targetChatId = ws?.activeChatId ?? null;
        const targetComparisonId = ws?.activeComparisonId ?? null;

        set({
            activeMode: mode,
            activeChatId: targetChatId,
            activeComparisonId: targetComparisonId,
        });
        localStorage.setItem('uni-active-mode', mode);

        if (targetChatId) {
            get().setActiveChat(targetChatId);
        }
    },

    loadModeSettings: async () => {
        try {
            const list = await invoke<ModeSettingRow[]>("get_mode_settings");
            set({ modeSettings: list ?? [] });
        } catch (e) {
            console.error("Failed to load mode settings:", e);
        }
    },

    updateModeSetting: async (mode, enabled, sortOrder, config) => {
        try {
            await invoke("update_mode_settings", { mode, enabled, sortOrder, config });
            await get().loadModeSettings();
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Comparisons ───────────────────────────────────────

    loadComparisons: async () => {
        try {
            const list = await invoke<Comparison[]>("get_all_comparisons");
            set({ comparisons: list ?? [] });
        } catch (e) {
            console.error("Failed to load comparisons:", e);
        }
    },

    setActiveComparison: async (id: string) => {
        const { activeMode, modeState } = get();
        const newModeState = {
            ...modeState,
            [activeMode]: { ...modeState[activeMode], activeComparisonId: id },
        };
        set({ activeComparisonId: id, modeState: newModeState, compareStreamingLeft: false, compareStreamingRight: false });
        localStorage.setItem('uni-mode-state', JSON.stringify(newModeState));
        try {
            const msgs = await invoke<ComparisonMessage[]>("get_comparison_messages", {
                comparisonId: id,
            });
            set({ comparisonMessages: msgs ?? [] });
        } catch (e) {
            console.error("Failed to load comparison messages:", e);
        }
    },

    createComparison: async (
        leftProviderId,
        leftModel,
        rightProviderId,
        rightModel,
        leftSystemPrompt,
        rightSystemPrompt,
    ) => {
        try {
            const comparison = await invoke<Comparison>("create_comparison", {
                leftProviderId,
                leftModel,
                rightProviderId,
                rightModel,
                leftSystemPrompt: leftSystemPrompt ?? null,
                rightSystemPrompt: rightSystemPrompt ?? null,
            });
            const { activeMode, modeState } = get();
            const newModeState = {
                ...modeState,
                [activeMode]: { ...modeState[activeMode], activeComparisonId: comparison.id },
            };
            set((state) => ({
                comparisons: [comparison, ...state.comparisons],
                activeComparisonId: comparison.id,
                modeState: newModeState,
                comparisonMessages: [],
                compareStreamingLeft: false,
                compareStreamingRight: false,
            }));
            localStorage.setItem('uni-mode-state', JSON.stringify(newModeState));
            notify.success(i18n.t("compare.comparisonCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteComparison: async (id: string) => {
        try {
            await invoke("delete_comparison", { comparisonId: id });
            const { activeMode, modeState } = get();
            const wasActive = get().activeComparisonId === id;
            const remaining = get().comparisons.filter((c) => c.id !== id);

            if (!wasActive) {
                set({ comparisons: remaining });
            } else {
                const nextId = remaining[0]?.id ?? null;
                const newModeState = {
                    ...modeState,
                    [activeMode]: { ...modeState[activeMode], activeComparisonId: nextId },
                };
                set({
                    comparisons: remaining,
                    activeComparisonId: nextId,
                    modeState: newModeState,
                    comparisonMessages: [],
                    compareStreamingLeft: false,
                    compareStreamingRight: false,
                });
                localStorage.setItem('uni-mode-state', JSON.stringify(newModeState));
            }

            if (wasActive) {
                const { activeComparisonId } = get();
                if (activeComparisonId) {
                    get().setActiveComparison(activeComparisonId);
                } else {
                    set({ currentView: "chat" });
                }
            }
            notify.info(i18n.t("compare.comparisonDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateComparisonTitle: async (id: string, title: string) => {
        try {
            await invoke("update_comparison_title", { comparisonId: id, title });
            set((state) => ({
                comparisons: state.comparisons.map((c) =>
                    c.id === id ? { ...c, title } : c
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    sendComparisonMessage: async (content: string) => {
        content = await resolveInjections(content);

        const { activeComparisonId, comparisons, settings, customProviders } = get();
        if (!activeComparisonId) return;

        const comparison = comparisons.find((c) => c.id === activeComparisonId);
        if (!comparison) return;

        const resolveP = (providerId: string) => resolveProvider(providerId, settings, customProviders);
        const leftResolved = resolveP(comparison.leftProviderId);
        const rightResolved = resolveP(comparison.rightProviderId);
        if (!leftResolved || !rightResolved) {
            notify.error(i18n.t("notifications.providerNotFound"));
            return;
        }
        if (comparison.leftProviderId === "openrouter" && !leftResolved.apiKey?.trim()) {
            notify.warning(i18n.t("notifications.apiKeyNotSet"));
            return;
        }
        if (comparison.rightProviderId === "openrouter" && !rightResolved.apiKey?.trim()) {
            notify.warning(i18n.t("notifications.apiKeyNotSet"));
            return;
        }

        await get().loadModels();

        const { draftAttachments, models } = get();
        const leftModel = models.find(m => m.id === comparison.leftModel);
        const rightModel = models.find(m => m.id === comparison.rightModel);
        const leftSupportsToolUse = leftModel?.supportsToolUse ?? true;
        const rightSupportsToolUse = rightModel?.supportsToolUse ?? true;

        set({ compareStreamingLeft: true, compareStreamingRight: true, compareToolCallsLeft: [], compareToolCallsRight: [] });

        const unlisteners: UnlistenFn[] = [];
        let unlistenTitle: UnlistenFn | null = null;

        try {
            unlisteners.push(
                await listen<{ userMessageId: string; leftAssistantId: string; rightAssistantId: string; content: string; timestamp: number; comparisonId: string; leftModel: string; rightModel: string; hasAttachments?: boolean }>(
                    "comparison-messages-saved",
                    (event) => {
                        const p = event.payload;
                        if (p.comparisonId !== activeComparisonId) return;
                        const userMsg: ComparisonMessage = {
                            id: p.userMessageId,
                            comparisonId: p.comparisonId,
                            role: "user",
                            side: null,
                            content: p.content,
                            timestamp: p.timestamp,
                            promptTokens: 0,
                            completionTokens: 0,
                            cost: 0,
                            hasAttachments: p.hasAttachments ? 1 : 0,
                        };
                        const leftMsg: ComparisonMessage = {
                            id: p.leftAssistantId,
                            comparisonId: p.comparisonId,
                            role: "assistant",
                            side: "left",
                            content: "",
                            timestamp: p.timestamp + 1,
                            model: p.leftModel,
                            promptTokens: 0,
                            completionTokens: 0,
                            cost: 0,
                        };
                        const rightMsg: ComparisonMessage = {
                            id: p.rightAssistantId,
                            comparisonId: p.comparisonId,
                            role: "assistant",
                            side: "right",
                            content: "",
                            timestamp: p.timestamp + 1,
                            model: p.rightModel,
                            promptTokens: 0,
                            completionTokens: 0,
                            cost: 0,
                        };
                        set((state) => ({
                            comparisonMessages: [...state.comparisonMessages, userMsg, leftMsg, rightMsg],
                        }));
                    }
                )
            );

            unlisteners.push(
                await listen<ComparisonStreamPayload>("comparison-stream", (event) => {
                    if (get().activeComparisonId !== activeComparisonId) return;
                    const { side, content: chunk } = event.payload;
                    set((state) => ({
                        comparisonMessages: state.comparisonMessages.map((m) =>
                            m.role === "assistant" && m.side === side &&
                            m === [...state.comparisonMessages].reverse().find(
                                (msg) => msg.role === "assistant" && msg.side === side
                            )
                                ? { ...m, content: m.content + chunk }
                                : m
                        ),
                    }));
                })
            );

            unlisteners.push(
                await listen<ComparisonStreamImagePayload>("comparison-stream-image", (event) => {
                    if (get().activeComparisonId !== activeComparisonId) return;
                    const { side, path, index } = event.payload;
                    const imageBlock: ContentBlock = {
                        type: "image",
                        path,
                        name: `image_${index}.png`,
                    };
                    set((state) => {
                        const msgs = [...state.comparisonMessages];
                        for (let i = msgs.length - 1; i >= 0; i--) {
                            if (msgs[i].role === "assistant" && msgs[i].side === side) {
                                const msg = msgs[i];
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
                                msgs[i] = { ...msg, content: JSON.stringify(blocks) };
                                break;
                            }
                        }
                        return { comparisonMessages: msgs };
                    });
                })
            );

            unlisteners.push(
                await listen<ComparisonStreamDonePayload>("comparison-stream-done", (event) => {
                    const { side, full_content } = event.payload;
                    const isCurrent = get().activeComparisonId === activeComparisonId;
                    set((state) => {
                        const update: Partial<{ compareStreamingLeft: boolean; compareStreamingRight: boolean; comparisonMessages: ComparisonMessage[] }> = {};
                        if (side === "left") update.compareStreamingLeft = false;
                        if (side === "right") update.compareStreamingRight = false;
                        if (isCurrent) {
                            const msgs = [...state.comparisonMessages];
                            for (let i = msgs.length - 1; i >= 0; i--) {
                                if (msgs[i].role === "assistant" && msgs[i].side === side) {
                                    msgs[i] = { ...msgs[i], content: full_content };
                                    break;
                                }
                            }
                            update.comparisonMessages = msgs;
                        }
                        return update;
                    });

                    const st = get();
                    if (!st.compareStreamingLeft && !st.compareStreamingRight) {
                        unlisteners.forEach((fn) => fn());
                        get().loadBalance();
                    }
                })
            );

            unlisteners.push(
                await listen<ComparisonStreamUsagePayload>("comparison-stream-usage", (event) => {
                    if (get().activeComparisonId !== activeComparisonId) return;
                    const { side, prompt_tokens, completion_tokens } = event.payload;
                    const { models } = get();
                    const comp = get().comparisons.find((c) => c.id === activeComparisonId);
                    const modelId = side === "left" ? comp?.leftModel : comp?.rightModel;
                    const modelInfo = modelId ? models.find((m) => m.id === modelId) : null;
                    let cost = 0;
                    if (modelInfo?.pricing) {
                        cost =
                            prompt_tokens * parseFloat(modelInfo.pricing.prompt) +
                            completion_tokens * parseFloat(modelInfo.pricing.completion);
                    }
                    set((state) => {
                        const msgs = [...state.comparisonMessages];
                        for (let i = msgs.length - 1; i >= 0; i--) {
                            if (msgs[i].role === "assistant" && msgs[i].side === side) {
                                msgs[i] = { ...msgs[i], promptTokens: prompt_tokens, completionTokens: completion_tokens, cost };
                                break;
                            }
                        }
                        return { comparisonMessages: msgs };
                    });

                    const lastMsg = [...get().comparisonMessages].reverse().find(
                        (m) => m.role === "assistant" && m.side === side
                    );
                    if (lastMsg) {
                        invoke("update_comparison_message_usage", {
                            messageId: lastMsg.id,
                            promptTokens: prompt_tokens,
                            completionTokens: completion_tokens,
                            cost,
                        }).catch(console.error);
                    }
                })
            );

            unlisteners.push(
                await listen<ComparisonStreamRetryPayload>("comparison-stream-retry", (event) => {
                    const { side, attempt, maxAttempts } = event.payload;
                    notify.warning(
                        `[${side}] ${i18n.t("notifications.retrying", { attempt, maxAttempts })}`,
                        `comparison-retry-${side}`
                    );
                })
            );

            unlisteners.push(
                await listen<ComparisonStreamErrorPayload>("comparison-stream-error", (event) => {
                    const { side, error } = event.payload;
                    notify.error(`[${side}] ${error}`);
                    set(() => {
                        const update: Record<string, unknown> = {};
                        if (side === "left") update.compareStreamingLeft = false;
                        if (side === "right") update.compareStreamingRight = false;
                        return update;
                    });

                    const st = get();
                    if (!st.compareStreamingLeft && !st.compareStreamingRight) {
                        unlisteners.forEach((fn) => fn());
                    }
                })
            );

            unlisteners.push(
                await listen<ComparisonToolCallPayload>("comparison-stream-tool-call", (event) => {
                    if (get().activeComparisonId !== activeComparisonId) return;
                    const { side, toolCallId, serverId, toolName, arguments: args } = event.payload;
                    set((state) => {
                        const key = side === "left" ? "compareToolCallsLeft" : "compareToolCallsRight";
                        return {
                            [key]: [...(state[key] || []), {
                                toolCallId,
                                serverId,
                                toolName,
                                arguments: args,
                                status: "calling" as const,
                            }],
                        };
                    });
                })
            );

            unlisteners.push(
                await listen<ComparisonToolResultPayload>("comparison-stream-tool-result", (event) => {
                    if (get().activeComparisonId !== activeComparisonId) return;
                    const { side, toolCallId, result, isError } = event.payload;
                    set((state) => {
                        const key = side === "left" ? "compareToolCallsLeft" : "compareToolCallsRight";
                        return {
                            [key]: (state[key] || []).map((tc: ToolCallInfo) =>
                                tc.toolCallId === toolCallId
                                    ? { ...tc, result, isError, status: isError ? "error" as const : "done" as const }
                                    : tc
                            ),
                        };
                    });
                })
            );

            unlistenTitle =
                await listen<{ comparisonId: string; title: string }>("comparison-title-updated", (event) => {
                    const { comparisonId, title } = event.payload;
                    set((state) => ({
                        comparisons: state.comparisons.map((c) =>
                            c.id === comparisonId ? { ...c, title } : c
                        ),
                    }));
                });

            const invokePayload: Record<string, unknown> = {
                comparisonId: activeComparisonId,
                content,
                leftBaseUrl: leftResolved.baseUrl,
                leftApiKey: leftResolved.apiKey,
                rightBaseUrl: rightResolved.baseUrl,
                rightApiKey: rightResolved.apiKey,
                leftTemperature: settings.temperature,
                leftMaxTokens: settings.max_tokens,
                rightTemperature: settings.temperature,
                rightMaxTokens: settings.max_tokens,
                leftSupportsToolUse,
                rightSupportsToolUse,
            };
            if (draftAttachments.length > 0) {
                invokePayload.attachments = draftAttachments.map((a) => ({
                    name: a.name,
                    mimeType: a.mimeType,
                    data: a.data,
                }));
            }
            await invoke("stream_comparison_responses", invokePayload);
            set({ draftAttachments: [] });
        } catch (e) {
            set({ compareStreamingLeft: false, compareStreamingRight: false });
            notify.error(String(e));
            unlisteners.forEach((fn) => fn());
        } finally {
            // Title event arrives after both streams complete, give it time to process
            setTimeout(() => { unlistenTitle?.(); }, 500);
        }
    },

    stopComparisonGeneration: async () => {
        try {
            await invoke("stop_comparison_generation");
        } catch (e) {
            console.error("Failed to stop comparison generation:", e);
        }
    },

    // ── Telegram ──────────────────────────────────────────

    startTelegramBot: async () => {
        try {
            await invoke("start_telegram_bot");
            await get().getTelegramStatus();
        } catch (e) {
            notify.error(String(e));
        }
    },

    stopTelegramBot: async () => {
        try {
            await invoke("stop_telegram_bot");
            await get().getTelegramStatus();
        } catch (e) {
            notify.error(String(e));
        }
    },

    getTelegramStatus: async () => {
        try {
            const status = await invoke<{ running: boolean; botUsername: string | null; authorizedUser: string | null }>("get_telegram_status");
            set({ telegramStatus: status });
        } catch (e) {
            console.error("Failed to get telegram status:", e);
        }
    },

    authorizeTelegramUser: async (userId: number, firstName: string, lastName: string | null, username: string | null) => {
        try {
            await invoke("authorize_telegram_user", { userId, firstName, lastName, username });
            await get().getTelegramStatus();
        } catch (e) {
            notify.error(String(e));
        }
    },

    revokeTelegramUser: async () => {
        try {
            await invoke("revoke_telegram_user");
            await get().getTelegramStatus();
        } catch (e) {
            notify.error(String(e));
        }
    },

    validateTelegramToken: async (token: string) => {
        const result = await invoke<{ username: string; firstName: string }>("validate_telegram_token", { token });
        return result;
    },
});

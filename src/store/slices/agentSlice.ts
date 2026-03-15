import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import i18n from "../../i18n";
import { notify } from "../../utils/notify";
import { resolveInjections } from "../../utils/injections";
import type {
    AgentStatus,
    AgentMemory,
    AgentRun,
    AgentRunStartedPayload,
    AgentStepPayload,
    AgentRunFinishedPayload,
    AgentRunLimitPayload,
    Skill,
    AgentPlan,
    AgentTask,
    PlanWithTasks,
    WorkspaceArtifact,
    SubAgentInfo,
    ScheduledTask,
    StreamPayload,
    StreamDonePayload,
    StreamErrorPayload,
    StreamUsagePayload,
    ToolCallEvent,
    ToolResultEvent,
    ToolCallInfo,
    MessageWithSiblings,
} from "../../types";
import { reloadChatMessages } from "../helpers";
import type { ChatState } from "../chatStore";

export interface AgentSlice {
    // State
    agentRunId: string | null;
    currentAgentRun: AgentRun | null;
    agentStatus: AgentStatus;
    agentIteration: number;
    agentMaxIterations: number;
    agentToolCalls: ToolCallInfo[];
    agentMemories: AgentMemory[];
    agentMemoriesLoading: boolean;
    skills: Skill[];
    chatSkills: Skill[];
    workspaceArtifacts: WorkspaceArtifact[];
    showWorkspacePanel: boolean;
    activeSubAgents: Record<string, SubAgentInfo>;
    activePlan: PlanWithTasks | null;
    planLoading: boolean;
    planGenerating: boolean;
    showPlanPanel: boolean;
    allPlans: AgentPlan[];
    scheduledTasks: ScheduledTask[];

    // Actions — Agent
    sendAgentMessage: (content: string) => Promise<void>;
    cancelAgentRun: () => Promise<void>;
    cancelSubAgentRun: (runId: string) => Promise<void>;
    resumeAgentRun: () => Promise<void>;

    // Actions — Agent Memory
    loadAgentMemories: (category?: string, searchText?: string) => Promise<void>;
    createAgentMemory: (content: string, category: string) => Promise<void>;
    updateAgentMemory: (id: string, updates: { content?: string; category?: string; isPinned?: boolean }) => Promise<void>;
    deleteAgentMemory: (id: string) => Promise<void>;
    deleteAllAgentMemories: () => Promise<void>;

    // Actions — Workspace
    setShowWorkspacePanel: (show: boolean) => void;
    loadWorkspaceArtifacts: (chatId: string) => Promise<void>;
    createWorkspaceArtifact: (chatId: string, name: string, contentType: string, content: string) => Promise<void>;
    updateWorkspaceArtifact: (id: string, name?: string, content?: string) => Promise<void>;
    deleteWorkspaceArtifact: (id: string) => Promise<void>;

    // Actions — Scheduler
    loadScheduledTasks: () => Promise<void>;
    createScheduledTask: (data: { name: string; prompt: string; cronExpression: string; model?: string; skillId?: string; projectId?: string; deliverTelegram: boolean; deliverDesktopNotification: boolean }) => Promise<void>;
    updateScheduledTask: (id: string, updates: Record<string, unknown>) => Promise<void>;
    deleteScheduledTask: (id: string) => Promise<void>;
    toggleScheduledTask: (id: string, enabled: boolean) => Promise<void>;
    runScheduledTaskNow: (id: string) => Promise<void>;

    // Actions — Sub-Agents
    addSubAgent: (data: SubAgentInfo) => void;
    updateSubAgent: (agentRunId: string, data: Partial<SubAgentInfo>) => void;
    clearSubAgents: (parentRunId: string) => void;

    // Actions — Skills
    loadSkills: () => Promise<void>;
    createSkill: (name: string, description: string, icon: string, content: string, triggerDescription: string, requiredTools: string) => Promise<void>;
    updateSkill: (id: string, updates: { name?: string; description?: string; icon?: string; content?: string; triggerDescription?: string; requiredTools?: string; enabled?: boolean; sortOrder?: number }) => Promise<void>;
    deleteSkill: (id: string) => Promise<void>;
    loadChatSkills: (chatId: string) => Promise<void>;
    attachSkillToChat: (chatId: string, skillId: string) => Promise<void>;
    detachSkillFromChat: (chatId: string, skillId: string) => Promise<void>;

    // Actions — Plans
    setShowPlanPanel: (show: boolean) => void;
    generatePlan: (chatId: string, goal: string) => Promise<void>;
    loadPlanForChat: (chatId: string) => Promise<void>;
    loadAllPlans: () => Promise<void>;
    approvePlan: (planId: string, executionMode: string) => Promise<void>;
    startPlanExecution: (planId: string) => Promise<void>;
    executeSingleTask: (taskId: string) => Promise<string>;
    updatePlanTask: (taskId: string, updates: { title?: string; description?: string; status?: string }) => Promise<void>;
    deletePlan: (planId: string) => Promise<void>;
    initPlanListeners: () => Promise<void>;
    initWorkspaceListeners: () => Promise<void>;
    initSubAgentListeners: () => Promise<void>;
}

type Set = (partial: Partial<ChatState> | ((state: ChatState) => Partial<ChatState>)) => void;
type Get = () => ChatState;

export const createAgentSlice = (set: Set, get: Get): AgentSlice => ({
    // ── State ──────────────────────────────────────────────

    agentRunId: null,
    currentAgentRun: null,
    agentStatus: "idle" as AgentStatus,
    agentIteration: 0,
    agentMaxIterations: 25,
    agentToolCalls: [],
    agentMemories: [],
    agentMemoriesLoading: false,
    skills: [],
    chatSkills: [],
    workspaceArtifacts: [],
    showWorkspacePanel: false,
    activeSubAgents: {},
    activePlan: null,
    planLoading: false,
    planGenerating: false,
    showPlanPanel: false,
    allPlans: [],
    scheduledTasks: [],

    // ─── Agent actions ───────────────────────────────────────────

    sendAgentMessage: async (content: string) => {
        const { activeChatId, activeMode, settings, chats } = get();
        if (activeMode !== "assistant") return;

        let chatId = activeChatId;
        if (!chatId) {
            await get().createChat();
            chatId = get().activeChatId;
        }
        if (!chatId) return;

        const chat = chats.find((c) => c.id === chatId);
        const model = chat?.model || settings.model;
        const systemPrompt = chat?.systemPrompt || "";
        const providerId = chat?.providerId || "openrouter";

        // Resolve provider credentials
        let apiKey = settings.api_key;
        let baseUrl: string | undefined;

        if (providerId === "ollama") {
            apiKey = "";
            baseUrl = settings.ollamaUrl || "http://localhost:11434/v1";
        } else if (providerId !== "openrouter") {
            const cp = get().customProviders.find((p) => p.id === providerId);
            if (cp) {
                apiKey = cp.apiKey;
                baseUrl = cp.baseUrl;
            }
        }

        // Resolve injections in content
        const resolvedContent = await resolveInjections(content);

        // Get parent message id (last message in chat for branching)
        const chatMessages = chat?.messages ?? [];
        const parentId = chatMessages.length > 0 ? chatMessages[chatMessages.length - 1].id : undefined;

        const userMessageId = crypto.randomUUID();
        const assistantMessageId = crypto.randomUUID();

        // Optimistically add user message to UI
        const now = Math.floor(Date.now() / 1000);
        const userMessage: MessageWithSiblings = {
            id: userMessageId,
            role: "user",
            content: resolvedContent,
            parentId,
            timestamp: now,
            model: "",
            agentStep: 0,
            siblingCount: 1,
            siblingIndex: 0,
            siblingIds: [userMessageId],
            siblings: [],
        };
        const assistantPlaceholder: MessageWithSiblings = {
            id: assistantMessageId,
            role: "assistant",
            content: "",
            parentId: userMessageId,
            timestamp: now + 1,
            model,
            siblingCount: 1,
            siblingIndex: 0,
            siblingIds: [assistantMessageId],
            siblings: [],
        };
        set((state) => ({
            chats: state.chats.map((c) =>
                c.id === chatId ? { ...c, messages: [...c.messages, userMessage, assistantPlaceholder] } : c
            ),
            isStreaming: true,
            activeToolCalls: [],
            agentToolCalls: [],
        }));

        // Set up agent event listeners
        const unlisteners: UnlistenFn[] = [];
        // Closure-scoped accumulator — survives Zustand state reloads
        let agentStreamAccum = "";
        let currentAsstId = assistantMessageId;

        try {
            unlisteners.push(
                await listen<AgentRunStartedPayload>("agent-run-started", async (event) => {
                    set({
                        agentRunId: event.payload.runId,
                        agentStatus: "running",
                        agentIteration: 0,
                        agentMaxIterations: event.payload.maxIterations,
                    });
                    try {
                        const runs = await invoke<AgentRun[]>("get_agent_runs", { chatId });
                        const run = runs?.find((r) => r.id === event.payload.runId) ?? null;
                        set({ currentAgentRun: run });
                    } catch {
                        set({ currentAgentRun: null });
                    }
                })
            );

            unlisteners.push(
                await listen<AgentStepPayload>("agent-step-start", (event) => {
                    const iteration = event.payload.iteration;
                    if (iteration > 1) {
                        // New iteration — reset accumulator and create new placeholder ID
                        agentStreamAccum = "";
                        currentAsstId = crypto.randomUUID();
                    }
                    set({ agentIteration: iteration });
                })
            );

            unlisteners.push(
                await listen<StreamPayload>("agent-stream", (event) => {
                    agentStreamAccum += event.payload.content;
                    const fullContent = agentStreamAccum;
                    const asstId = currentAsstId;
                    set((state) => ({
                        chats: state.chats.map((c) => {
                            if (c.id !== chatId) return c;
                            // Try to find current assistant message by ID
                            const exists = c.messages.some((m) => m.id === asstId);
                            if (exists) {
                                return {
                                    ...c,
                                    messages: c.messages.map((msg) =>
                                        msg.id === asstId
                                            ? { ...msg, content: fullContent }
                                            : msg
                                    ),
                                };
                            }
                            // Not found (removed by reloadChatMessages or new iteration) — create placeholder
                            const lastMsg = c.messages[c.messages.length - 1];
                            return {
                                ...c,
                                messages: [
                                    ...c.messages,
                                    {
                                        id: asstId,
                                        role: "assistant" as const,
                                        content: fullContent,
                                        parentId: lastMsg?.id,
                                        timestamp: Math.floor(Date.now() / 1000),
                                        model: model,
                                        siblingCount: 1,
                                        siblingIndex: 0,
                                        siblingIds: [asstId],
                                        siblings: [],
                                    },
                                ],
                            };
                        }),
                    }));
                })
            );

            unlisteners.push(
                await listen<StreamUsagePayload>("agent-stream-usage", async () => {
                    const { agentRunId: runId, activeChatId: cid } = get();
                    if (!runId || !cid) return;
                    try {
                        const runs = await invoke<AgentRun[]>("get_agent_runs", { chatId: cid });
                        const run = runs?.find((r) => r.id === runId) ?? null;
                        set({ currentAgentRun: run });
                    } catch {
                        // ignore
                    }
                })
            );

            unlisteners.push(
                await listen<ToolCallEvent>("agent-tool-call", (event) => {
                    const p = event.payload;
                    set((state) => ({
                        agentToolCalls: [
                            ...state.agentToolCalls,
                            {
                                toolCallId: p.toolCallId,
                                serverId: p.serverId,
                                toolName: p.toolName,
                                arguments: p.arguments,
                                status: "calling" as const,
                            },
                        ],
                    }));
                })
            );

            unlisteners.push(
                await listen<ToolResultEvent>("agent-tool-result", (event) => {
                    const p = event.payload;
                    set((state) => ({
                        agentToolCalls: state.agentToolCalls.map((tc) =>
                            tc.toolCallId === p.toolCallId
                                ? { ...tc, result: p.result, isError: p.isError, status: (p.isError ? "error" : "done") as "error" | "done" }
                                : tc
                        ),
                    }));
                })
            );

            unlisteners.push(
                await listen<StreamDonePayload>("agent-stream-done", () => {
                    agentStreamAccum = "";
                    // Don't reload here — agent-run-finished will reload after DB commit
                    set({
                        isStreaming: false,
                        agentToolCalls: [],
                    });
                })
            );

            unlisteners.push(
                await listen<StreamErrorPayload>("agent-stream-error", (event) => {
                    notify.error(event.payload.error);
                })
            );

            unlisteners.push(
                await listen<AgentStepPayload>("agent-step-pause", () => {
                    set({ agentStatus: "paused" });
                })
            );

            unlisteners.push(
                await listen<AgentStepPayload>("agent-step-done", async (event) => {
                    const cid = get().activeChatId;
                    if (cid) reloadChatMessages(cid, get, set);
                    set({ agentIteration: event.payload.iteration, agentToolCalls: [] });
                    const { agentRunId: runId } = get();
                    if (runId && cid) {
                        try {
                            const runs = await invoke<AgentRun[]>("get_agent_runs", { chatId: cid });
                            const run = runs?.find((r) => r.id === runId) ?? null;
                            set({ currentAgentRun: run });
                        } catch {
                            // ignore
                        }
                    }
                })
            );

            unlisteners.push(
                await listen<AgentRunFinishedPayload>("agent-run-finished", () => {
                    agentStreamAccum = "";
                    set({
                        isStreaming: false,
                        agentStatus: "idle",
                        agentRunId: null,
                        currentAgentRun: null,
                        agentToolCalls: [],
                        agentIteration: 0,
                        agentMaxIterations: 25,
                    });
                    // 2. Reload messages with small delay to ensure DB commit is flushed
                    const cid = get().activeChatId;
                    if (cid) {
                        setTimeout(() => {
                            reloadChatMessages(cid, get, set);
                        }, 150);
                    }
                    // Cleanup listeners
                    for (const unlisten of unlisteners) unlisten();
                })
            );

            unlisteners.push(
                await listen<AgentRunLimitPayload>("agent-run-limit", () => {
                    notify.warning(i18n.t("agent.maxIterationsReached"));
                })
            );

            // Invoke the backend command
            await invoke<string>("send_agent_message", {
                chatId,
                content: resolvedContent,
                model,
                baseUrl: baseUrl ?? null,
                apiKey,
                systemPrompt: systemPrompt || null,
                parentId: parentId ?? null,
                userMessageId,
                assistantMessageId,
                maxIterations: null,
                autoMode: null,
                temperature: chat?.temperature ?? null,
                maxTokens: chat?.maxTokens ?? null,
                topP: chat?.topP ?? null,
                topK: chat?.topK ?? null,
                frequencyPenalty: chat?.frequencyPenalty ?? null,
                presencePenalty: chat?.presencePenalty ?? null,
                supportsToolUse: null,
            });

            // Update chat title if first message
            if (chatMessages.length === 0) {
                const trimmed = resolvedContent.trim();
                const title = trimmed.substring(0, 60) + (trimmed.length > 60 ? "…" : "");
                await invoke("update_chat_title", { chatId, title });
                set((state) => ({
                    chats: state.chats.map((c) => (c.id === chatId ? { ...c, title } : c)),
                }));
            }
        } catch (e) {
            notify.error(String(e));
            set({ isStreaming: false, agentStatus: "idle" as AgentStatus, agentRunId: null });
            for (const unlisten of unlisteners) unlisten();
        }
    },

    cancelAgentRun: async () => {
        const { agentRunId } = get();
        if (!agentRunId) return;
        try {
            await invoke("cancel_agent_run", { runId: agentRunId });
        } catch (e) {
            notify.error(String(e));
        }
    },

    cancelSubAgentRun: async (runId: string) => {
        try {
            await invoke("cancel_agent_run", { runId });
        } catch (e) {
            notify.error(String(e));
        }
    },

    resumeAgentRun: async () => {
        const { agentRunId } = get();
        if (!agentRunId) return;
        try {
            set({ agentStatus: "running" as AgentStatus, isStreaming: true });
            await invoke("resume_agent_run", { runId: agentRunId });
        } catch (e) {
            notify.error(String(e));
            set({ isStreaming: false, agentStatus: "idle" });
        }
    },

    // ─── Agent Memory ──────────────────────────────────────────

    loadAgentMemories: async (category, searchText) => {
        set({ agentMemoriesLoading: true });
        try {
            const list = await invoke<AgentMemory[]>("list_agent_memories", {
                category: category || null,
                searchText: searchText || null,
            });
            set({ agentMemories: list ?? [], agentMemoriesLoading: false });
        } catch (e) {
            console.error("Failed to load agent memories:", e);
            set({ agentMemoriesLoading: false });
        }
    },

    createAgentMemory: async (content, category) => {
        try {
            const memory = await invoke<AgentMemory>("create_agent_memory", { content, category });
            set((state) => ({ agentMemories: [memory, ...state.agentMemories] }));
            notify.success(i18n.t("memory.savedToMemory"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateAgentMemory: async (id, updates) => {
        try {
            const memory = await invoke<AgentMemory>("update_agent_memory", { id, ...updates });
            set((state) => ({
                agentMemories: state.agentMemories.map((m) => (m.id === id ? memory : m)),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteAgentMemory: async (id) => {
        try {
            await invoke("delete_agent_memory", { id });
            set((state) => ({
                agentMemories: state.agentMemories.filter((m) => m.id !== id),
            }));
            notify.success(i18n.t("memory.memoryDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteAllAgentMemories: async () => {
        try {
            await invoke("delete_all_agent_memories");
            set({ agentMemories: [] });
            notify.success(i18n.t("memory.allMemoriesDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ─── Workspace ────────────────────────────────────────────────

    setShowWorkspacePanel: (show) => set({ showWorkspacePanel: show }),

    loadWorkspaceArtifacts: async (chatId) => {
        try {
            const chat = get().chats.find((c) => c.id === chatId);
            const projectId = chat?.projectId ?? undefined;
            const list = await invoke<WorkspaceArtifact[]>("list_workspace_artifacts", { chatId, projectId });
            set({ workspaceArtifacts: list ?? [] });
        } catch (e) {
            console.error("Failed to load workspace artifacts:", e);
        }
    },

    createWorkspaceArtifact: async (chatId, name, contentType, content) => {
        try {
            const chat = get().chats.find((c) => c.id === chatId);
            const projectId = chat?.projectId ?? undefined;
            const artifact = await invoke<WorkspaceArtifact>("create_workspace_artifact", {
                chatId, projectId, name, contentType, content,
            });
            set((state) => ({ workspaceArtifacts: [artifact, ...state.workspaceArtifacts] }));
            notify.success(i18n.t("workspace.artifactCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateWorkspaceArtifact: async (id, name, content) => {
        try {
            await invoke("update_workspace_artifact", { id, name, content });
            set((state) => ({
                workspaceArtifacts: state.workspaceArtifacts.map((a) =>
                    a.id === id
                        ? { ...a, ...(name !== undefined && { name }), ...(content !== undefined && { content }) }
                        : a
                ),
            }));
            notify.success(i18n.t("workspace.artifactUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteWorkspaceArtifact: async (id) => {
        try {
            await invoke("delete_workspace_artifact", { id });
            set((state) => ({
                workspaceArtifacts: state.workspaceArtifacts.filter((a) => a.id !== id),
            }));
            notify.success(i18n.t("workspace.artifactDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ─── Scheduler ─────────────────────────────────────────────

    loadScheduledTasks: async () => {
        try {
            const list = await invoke<ScheduledTask[]>("list_scheduled_tasks");
            set({ scheduledTasks: list ?? [] });
        } catch (e) {
            console.error("Failed to load scheduled tasks:", e);
        }
    },

    createScheduledTask: async (data) => {
        try {
            const task = await invoke<ScheduledTask>("create_scheduled_task", data);
            set((state) => ({ scheduledTasks: [task, ...state.scheduledTasks] }));
            notify.success(i18n.t("scheduler.taskCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateScheduledTask: async (id, updates) => {
        try {
            await invoke("update_scheduled_task", { id, ...updates });
            const list = await invoke<ScheduledTask[]>("list_scheduled_tasks");
            set({ scheduledTasks: list ?? [] });
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteScheduledTask: async (id) => {
        try {
            await invoke("delete_scheduled_task", { id });
            set((state) => ({
                scheduledTasks: state.scheduledTasks.filter((t) => t.id !== id),
            }));
            notify.success(i18n.t("scheduler.taskDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    toggleScheduledTask: async (id, enabled) => {
        try {
            await invoke("toggle_scheduled_task", { id, enabled });
            const list = await invoke<ScheduledTask[]>("list_scheduled_tasks");
            set({ scheduledTasks: list ?? [] });
        } catch (e) {
            notify.error(String(e));
        }
    },

    runScheduledTaskNow: async (id) => {
        try {
            await invoke("run_scheduled_task_now", { id });
            notify.success(i18n.t("scheduler.taskQueued"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ─── Sub-Agents ─────────────────────────────────────────────

    addSubAgent: (data) => {
        set((state) => ({
            activeSubAgents: { ...state.activeSubAgents, [data.agentRunId]: data },
        }));
    },

    updateSubAgent: (agentRunId, data) => {
        set((state) => {
            const existing = state.activeSubAgents[agentRunId];
            if (!existing) return state;
            return {
                activeSubAgents: {
                    ...state.activeSubAgents,
                    [agentRunId]: { ...existing, ...data },
                },
            };
        });
    },

    clearSubAgents: (parentRunId) => {
        set((state) => {
            const filtered: Record<string, SubAgentInfo> = {};
            for (const [k, v] of Object.entries(state.activeSubAgents)) {
                if (v.parentRunId !== parentRunId) filtered[k] = v;
            }
            return { activeSubAgents: filtered };
        });
    },

    // ─── Skills ──────────────────────────────────────────────────

    loadSkills: async () => {
        try {
            const list = await invoke<Skill[]>("list_skills");
            set({ skills: list ?? [] });
        } catch (e) {
            console.error("Failed to load skills:", e);
        }
    },

    createSkill: async (name, description, icon, content, triggerDescription, requiredTools) => {
        try {
            const skill = await invoke<Skill>("create_skill", {
                name, description, icon, content, triggerDescription, requiredTools,
            });
            set((state) => ({ skills: [...state.skills, skill] }));
            notify.success(i18n.t("skills.skillCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateSkill: async (id, updates) => {
        try {
            await invoke("update_skill", { id, ...updates });
            const list = await invoke<Skill[]>("list_skills");
            set({ skills: list ?? [] });
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteSkill: async (id) => {
        try {
            await invoke("delete_skill", { id });
            set((state) => ({
                skills: state.skills.filter((s) => s.id !== id),
            }));
            notify.success(i18n.t("skills.skillDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    loadChatSkills: async (chatId) => {
        try {
            const list = await invoke<Skill[]>("get_chat_skills", { chatId });
            set({ chatSkills: list ?? [] });
        } catch (e) {
            console.error("Failed to load chat skills:", e);
        }
    },

    attachSkillToChat: async (chatId, skillId) => {
        try {
            await invoke("attach_skill_to_chat", { chatId, skillId });
            const list = await invoke<Skill[]>("get_chat_skills", { chatId });
            set({ chatSkills: list ?? [] });
        } catch (e) {
            notify.error(String(e));
        }
    },

    detachSkillFromChat: async (chatId, skillId) => {
        try {
            await invoke("detach_skill_from_chat", { chatId, skillId });
            const list = await invoke<Skill[]>("get_chat_skills", { chatId });
            set({ chatSkills: list ?? [] });
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ─── Plans ──────────────────────────────────────────────────

    setShowPlanPanel: (show) => set({ showPlanPanel: show }),

    generatePlan: async (chatId, goal) => {
        set({ planGenerating: true });
        try {
            const { chats, settings, customProviders } = get();
            const chat = chats.find((c) => c.id === chatId);
            const model = chat?.model || settings.model;
            const providerId = chat?.providerId || "openrouter";
            let apiKey = settings.api_key;
            let baseUrl: string | undefined;
            if (providerId === "ollama") {
                apiKey = "";
                baseUrl = settings.ollamaUrl || "http://localhost:11434/v1";
            } else if (providerId !== "openrouter") {
                const cp = customProviders.find((p) => p.id === providerId);
                if (cp) { apiKey = cp.apiKey; baseUrl = cp.baseUrl; }
            }
            const result = await invoke<PlanWithTasks>("generate_plan", { chatId, goal, model, baseUrl, apiKey });
            set({ activePlan: result, planGenerating: false, showPlanPanel: true });
            notify.success(i18n.t("plans.planCreated"));
        } catch (e) {
            set({ planGenerating: false });
            notify.error(String(e));
        }
    },

    loadPlanForChat: async (chatId) => {
        try {
            const plans = await invoke<AgentPlan[]>("get_plans_for_chat", { chatId });
            if (plans && plans.length > 0) {
                const full = await invoke<PlanWithTasks>("get_plan", { planId: plans[0].id });
                const show = ["draft", "approved", "running"].includes(full.plan.status);
                set({ activePlan: full, showPlanPanel: show });
            } else {
                set({ activePlan: null, showPlanPanel: false });
            }
        } catch (e) {
            console.error("Failed to load plan for chat:", e);
            set({ activePlan: null, showPlanPanel: false });
        }
    },

    loadAllPlans: async () => {
        try {
            const { chats } = get();
            const allPlans: AgentPlan[] = [];
            for (const chat of chats) {
                const plans = await invoke<AgentPlan[]>("get_plans_for_chat", { chatId: chat.id });
                if (plans) allPlans.push(...plans);
            }
            set({ allPlans });
        } catch (e) {
            console.error("Failed to load all plans:", e);
        }
    },

    approvePlan: async (planId, executionMode) => {
        try {
            const updated = await invoke<AgentPlan>("approve_plan", { planId, executionMode });
            set((state) => ({
                activePlan: state.activePlan && state.activePlan.plan.id === planId
                    ? { ...state.activePlan, plan: updated }
                    : state.activePlan,
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    startPlanExecution: async (planId) => {
        try {
            const { chats, activeChatId, settings, customProviders } = get();
            const chat = chats.find((c) => c.id === activeChatId);
            const model = chat?.model || settings.model;
            const providerId = chat?.providerId || "openrouter";
            let apiKey = settings.api_key;
            let baseUrl: string | undefined;
            if (providerId === "ollama") {
                apiKey = "";
                baseUrl = settings.ollamaUrl || "http://localhost:11434/v1";
            } else if (providerId !== "openrouter") {
                const cp = customProviders.find((p) => p.id === providerId);
                if (cp) { apiKey = cp.apiKey; baseUrl = cp.baseUrl; }
            }
            const systemPrompt = chat?.systemPrompt || undefined;
            const temperature = chat?.temperature ?? undefined;
            const maxTokens = chat?.maxTokens ?? undefined;
            const topP = chat?.topP ?? undefined;
            const topK = chat?.topK ?? undefined;
            const frequencyPenalty = chat?.frequencyPenalty ?? undefined;
            const presencePenalty = chat?.presencePenalty ?? undefined;
            await invoke("start_plan_execution", {
                planId, model, baseUrl, apiKey, systemPrompt,
                temperature, maxTokens, topP, topK, frequencyPenalty, presencePenalty,
            });
        } catch (e) {
            notify.error(String(e));
        }
    },

    executeSingleTask: async (taskId) => {
        try {
            const { chats, activeChatId, settings, customProviders } = get();
            const chat = chats.find((c) => c.id === activeChatId);
            const model = chat?.model || settings.model;
            const providerId = chat?.providerId || "openrouter";
            let apiKey = settings.api_key;
            let baseUrl: string | undefined;
            if (providerId === "ollama") {
                apiKey = "";
                baseUrl = settings.ollamaUrl || "http://localhost:11434/v1";
            } else if (providerId !== "openrouter") {
                const cp = customProviders.find((p) => p.id === providerId);
                if (cp) { apiKey = cp.apiKey; baseUrl = cp.baseUrl; }
            }
            const systemPrompt = chat?.systemPrompt || undefined;
            const temperature = chat?.temperature ?? undefined;
            const maxTokens = chat?.maxTokens ?? undefined;
            const topP = chat?.topP ?? undefined;
            const topK = chat?.topK ?? undefined;
            const frequencyPenalty = chat?.frequencyPenalty ?? undefined;
            const presencePenalty = chat?.presencePenalty ?? undefined;
            const runId = await invoke<string>("execute_single_task", {
                taskId, model, baseUrl, apiKey, systemPrompt,
                temperature, maxTokens, topP, topK, frequencyPenalty, presencePenalty,
            });
            return runId;
        } catch (e) {
            notify.error(String(e));
            return "";
        }
    },

    updatePlanTask: async (taskId, updates) => {
        try {
            const updated = await invoke<AgentTask>("update_task", {
                taskId,
                title: updates.title,
                description: updates.description,
                status: updates.status,
            });
            set((state) => ({
                activePlan: state.activePlan
                    ? {
                        ...state.activePlan,
                        tasks: state.activePlan.tasks.map((t) => t.id === taskId ? updated : t),
                    }
                    : null,
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deletePlan: async (planId) => {
        try {
            await invoke("delete_plan", { planId });
            set((state) => ({
                activePlan: state.activePlan?.plan.id === planId ? null : state.activePlan,
                showPlanPanel: state.activePlan?.plan.id === planId ? false : state.showPlanPanel,
                allPlans: state.allPlans.filter((p) => p.id !== planId),
            }));
            notify.success(i18n.t("plans.planDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    initPlanListeners: async () => {
        await listen<{ planId: string; taskId: string; status: string }>("plan-task-updated", (event) => {
            const { planId, taskId, status } = event.payload;
            set((state) => {
                if (!state.activePlan || state.activePlan.plan.id !== planId) return {};
                return {
                    activePlan: {
                        ...state.activePlan,
                        tasks: state.activePlan.tasks.map((t) =>
                            t.id === taskId ? { ...t, status } : t
                        ),
                    },
                };
            });
        });
        await listen<{ planId: string; status: string }>("plan-status-changed", (event) => {
            const { planId, status } = event.payload;
            set((state) => {
                if (!state.activePlan || state.activePlan.plan.id !== planId) return {};
                return {
                    activePlan: {
                        ...state.activePlan,
                        plan: { ...state.activePlan.plan, status },
                    },
                };
            });
        });
        await listen<{ planId: string; taskId: string; agentRunId: string }>("plan-task-started", (event) => {
            const { planId, taskId, agentRunId } = event.payload;
            set((state) => {
                if (!state.activePlan || state.activePlan.plan.id !== planId) return {};
                return {
                    activePlan: {
                        ...state.activePlan,
                        tasks: state.activePlan.tasks.map((t) =>
                            t.id === taskId ? { ...t, status: "running", agentRunId } : t
                        ),
                    },
                };
            });
        });
        await listen<{ planId: string; newTaskCount: number }>("plan-replanned", async (event) => {
            const { planId } = event.payload;
            const state = get();
            if (!state.activePlan || state.activePlan.plan.id !== planId) return;
            // Reload full plan from backend to get updated tasks
            try {
                const updated = await invoke<{ plan: import("../../types").AgentPlan; tasks: import("../../types").AgentTask[] }>("get_plan", { planId });
                set({ activePlan: updated });
            } catch (e) {
                console.error("Failed to reload plan after replan:", e);
            }
        });
    },

    initWorkspaceListeners: async () => {
        await listen<{ chatId: string; projectId?: string | null }>("workspace-changed", (event) => {
            const { chatId, projectId } = event.payload;
            const state = get();
            if (!state.activeChatId) return;
            // Refresh if this is our chat, or if we share the same project
            if (state.activeChatId === chatId) {
                void state.loadWorkspaceArtifacts(chatId);
            } else if (projectId) {
                const activeChat = state.chats.find((c) => c.id === state.activeChatId);
                if (activeChat?.projectId === projectId) {
                    void state.loadWorkspaceArtifacts(state.activeChatId);
                }
            }
        });
    },

    initSubAgentListeners: async () => {
        await listen<{ parentRunId: string; agentRunId: string; goal: string; model: string | null }>(
            "sub-agent-started",
            (event) => {
                const { parentRunId, agentRunId, goal, model } = event.payload;
                get().addSubAgent({
                    agentRunId,
                    parentRunId,
                    goal,
                    model,
                    status: "running",
                    iterations: 0,
                    maxIterations: 15,
                    cost: null,
                });
            }
        );
        await listen<{ parentRunId: string; agentRunId: string; status: string; iterations: number; cost: number | null }>(
            "sub-agent-finished",
            (event) => {
                const { agentRunId, status, iterations, cost } = event.payload;
                get().updateSubAgent(agentRunId, {
                    status: status as SubAgentInfo["status"],
                    iterations,
                    cost,
                });
            }
        );
    },
});

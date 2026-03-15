import { create } from "zustand";
import type {
    Attachment,
    Chat,
    ChatTemplate,
    Comparison,
    ComparisonMessage,
    Folder,
    Preset,
    Category,
    Snippet,
    AppSettings,
    CustomProvider,
    ModelInfo,
    OllamaLocalModel,
    BalanceInfo,
    McpServer,
    McpConnectionInfo,
    FsMcpConfig,
    FsAuditEntry,
    ToolCallInfo,
    ImageStyle,
    PromptLibraryItem,
    ModeSettingRow,
    ModeStateMap,
    AgentStatus,
    AgentMemory,
    AgentRun,
    Skill,
    AgentPlan,
    PlanWithTasks,
    WorkspaceArtifact,
    SubAgentInfo,
    ProjectSummary,
    ScheduledTask,
} from "../types";
import { createSnippetsSlice } from "./slices/snippetsSlice";
import { createSettingsSlice } from "./slices/settingsSlice";
import { createUiSlice } from "./slices/uiSlice";
import { createAgentSlice } from "./slices/agentSlice";
import { createChatSlice } from "./slices/chatSlice";

export interface ChatState {
    chats: Chat[];
    activeChatId: string | null;
    settings: AppSettings;
    presets: Preset[];
    folders: Folder[];
    projects: ProjectSummary[];
    activeProjectId: string | null;
    categories: Category[];
    snippets: Snippet[];
    templates: ChatTemplate[];
    customProviders: CustomProvider[];
    insertSnippetText: string | null;
    welcomeSnippets: Snippet[];
    isStreaming: boolean;
    isStopping: boolean;
    webSearchEnabled: boolean;
    isSearching: boolean;
    balance: BalanceInfo | null;
    draftAttachments: Attachment[];
    activeToolCalls: ToolCallInfo[];
    playingMessageId: string | null;
    imageStyles: ImageStyle[];
    telegramStatus: { running: boolean; botUsername: string | null; authorizedUser: string | null };
    startTelegramBot: () => Promise<void>;
    stopTelegramBot: () => Promise<void>;
    getTelegramStatus: () => Promise<void>;
    authorizeTelegramUser: (userId: number, firstName: string, lastName: string | null, username: string | null) => Promise<void>;
    revokeTelegramUser: () => Promise<void>;
    validateTelegramToken: (token: string) => Promise<{ username: string; firstName: string }>;

    currentView: "chat" | "settings" | "snippets" | "search" | "compare" | "comparisons" | "promptLibrary" | "memory" | "skills" | "plans" | "projectDashboard" | "scheduler";
    setView: (view: "chat" | "settings" | "snippets" | "search" | "compare" | "comparisons" | "promptLibrary" | "memory" | "skills" | "plans" | "projectDashboard" | "scheduler") => void;
    openProjectDashboard: (projectId: string) => void;
    scrollTargetId: string | null;
    setScrollTargetId: (id: string | null) => void;
    models: ModelInfo[];
    ollamaStatus: "unknown" | "available" | "unavailable";
    localOllamaModels: OllamaLocalModel[];
    ollamaPullProgress: { model: string; progress: number; status: string } | null;
    checkOllamaStatus: () => Promise<void>;
    loadLocalOllamaModels: () => Promise<void>;
    deleteOllamaModel: (modelName: string) => Promise<void>;
    pullOllamaModel: (modelName: string) => Promise<void>;
    clearOllamaPullProgress: () => void;
    initOllamaPullListeners: () => Promise<void>;
    initTtsListeners: () => Promise<void>;
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
    loadImageStyles: () => Promise<void>;
    createImageStyle: (name: string, promptSuffix: string) => Promise<void>;
    updateImageStyle: (id: string, name: string, promptSuffix: string) => Promise<void>;
    deleteImageStyle: (id: string) => Promise<void>;

    loadCategories: () => Promise<void>;
    createCategory: (name: string) => Promise<void>;
    updateCategory: (id: string, name: string) => Promise<void>;
    deleteCategory: (id: string) => Promise<void>;
    loadSnippets: () => Promise<void>;
    loadWelcomeSnippets: () => Promise<void>;
    createSnippet: (name: string, content: string, categoryId: string, showOnWelcome?: boolean) => Promise<void>;
    updateSnippet: (id: string, name: string, content: string, categoryId: string, showOnWelcome?: boolean) => Promise<void>;
    deleteSnippet: (id: string) => Promise<void>;
    setInsertSnippetText: (text: string | null) => void;

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
    loadFolders: () => Promise<void>;
    createFolder: (name: string, color: string | null) => Promise<void>;
    updateFolder: (id: string, name: string, color: string | null) => Promise<void>;
    deleteFolder: (id: string) => Promise<void>;
    reorderFolders: (ids: string[]) => Promise<void>;
    moveChatToFolder: (chatId: string, folderId: string | null) => Promise<void>;

    loadProjects: () => Promise<void>;
    createProject: (name: string, goal: string) => Promise<void>;
    updateProject: (id: string, updates: { name?: string; goal?: string; status?: string }) => Promise<void>;
    deleteProject: (id: string) => Promise<void>;
    archiveProject: (id: string) => Promise<void>;
    assignChatToProject: (chatId: string, projectId: string) => Promise<void>;
    removeChatFromProject: (chatId: string) => Promise<void>;
    setActiveProjectId: (id: string | null) => void;

    loadTemplates: () => Promise<void>;
    createTemplate: (params: Omit<ChatTemplate, "id" | "sortOrder" | "createdAt">) => Promise<void>;
    updateTemplate: (
        id: string,
        params: Omit<ChatTemplate, "id" | "sortOrder" | "createdAt">
    ) => Promise<void>;
    deleteTemplate: (id: string) => Promise<void>;
    reorderTemplates: (ids: string[]) => Promise<void>;
    createChatFromTemplate: (template: ChatTemplate) => Promise<void>;

    mcpServers: McpServer[];
    mcpConnections: McpConnectionInfo[];
    loadMcpServers: () => Promise<void>;
    loadMcpConnections: () => Promise<void>;
    addMcpServer: (name: string, command: string, args: string[], env: Record<string, string>) => Promise<void>;
    updateMcpServer: (id: string, name: string, command: string, args: string[], env: Record<string, string>, enabled: boolean) => Promise<void>;
    removeMcpServer: (id: string) => Promise<void>;
    toggleMcpServer: (id: string, enabled: boolean) => Promise<void>;
    connectMcpServer: (id: string) => Promise<void>;

    fsConfig: FsMcpConfig | null;
    fsAuditLog: FsAuditEntry[];
    loadFsConfig: () => Promise<void>;
    saveFsConfig: (config: FsMcpConfig) => Promise<void>;
    loadFsAuditLog: (limit?: number) => Promise<void>;
    clearFsAuditLog: () => Promise<void>;

    promptLibrary: PromptLibraryItem[];
    promptLibraryLoading: boolean;

    activeMode: string;
    modeState: ModeStateMap;
    modeSettings: ModeSettingRow[];

    // Agent state
    agentRunId: string | null;
    currentAgentRun: AgentRun | null;
    agentStatus: AgentStatus;
    agentIteration: number;
    agentMaxIterations: number;
    agentToolCalls: ToolCallInfo[];

    sendAgentMessage: (content: string) => Promise<void>;
    cancelAgentRun: () => Promise<void>;
    cancelSubAgentRun: (runId: string) => Promise<void>;
    resumeAgentRun: () => Promise<void>;

    // Agent Memory
    agentMemories: AgentMemory[];
    agentMemoriesLoading: boolean;
    loadAgentMemories: (category?: string, searchText?: string) => Promise<void>;
    createAgentMemory: (content: string, category: string) => Promise<void>;
    updateAgentMemory: (id: string, updates: { content?: string; category?: string; isPinned?: boolean }) => Promise<void>;
    deleteAgentMemory: (id: string) => Promise<void>;
    deleteAllAgentMemories: () => Promise<void>;

    // Skills
    skills: Skill[];
    chatSkills: Skill[];
    loadSkills: () => Promise<void>;
    createSkill: (name: string, description: string, icon: string, content: string, triggerDescription: string, requiredTools: string) => Promise<void>;
    updateSkill: (id: string, updates: { name?: string; description?: string; icon?: string; content?: string; triggerDescription?: string; requiredTools?: string; enabled?: boolean; sortOrder?: number }) => Promise<void>;
    deleteSkill: (id: string) => Promise<void>;
    loadChatSkills: (chatId: string) => Promise<void>;
    attachSkillToChat: (chatId: string, skillId: string) => Promise<void>;
    detachSkillFromChat: (chatId: string, skillId: string) => Promise<void>;

    fetchPromptLibrary: (category?: string, search?: string) => Promise<void>;
    createPromptLibraryItem: (data: { title: string; description: string; content: string; category: string }) => Promise<void>;
    updatePromptLibraryItem: (id: string, data: { title: string; description: string; content: string; category: string }) => Promise<void>;
    deletePromptLibraryItem: (id: string) => Promise<void>;

    // Workspace
    workspaceArtifacts: WorkspaceArtifact[];
    showWorkspacePanel: boolean;
    setShowWorkspacePanel: (show: boolean) => void;
    loadWorkspaceArtifacts: (chatId: string) => Promise<void>;
    createWorkspaceArtifact: (chatId: string, name: string, contentType: string, content: string) => Promise<void>;
    updateWorkspaceArtifact: (id: string, name?: string, content?: string) => Promise<void>;
    deleteWorkspaceArtifact: (id: string) => Promise<void>;

    // Sub-Agents
    activeSubAgents: Record<string, SubAgentInfo>;
    addSubAgent: (data: SubAgentInfo) => void;
    updateSubAgent: (agentRunId: string, data: Partial<SubAgentInfo>) => void;
    clearSubAgents: (parentRunId: string) => void;

    // Scheduler
    scheduledTasks: ScheduledTask[];
    loadScheduledTasks: () => Promise<void>;
    createScheduledTask: (data: { name: string; prompt: string; cronExpression: string; model?: string; skillId?: string; projectId?: string; deliverTelegram: boolean; deliverDesktopNotification: boolean }) => Promise<void>;
    updateScheduledTask: (id: string, updates: Record<string, unknown>) => Promise<void>;
    deleteScheduledTask: (id: string) => Promise<void>;
    toggleScheduledTask: (id: string, enabled: boolean) => Promise<void>;
    runScheduledTaskNow: (id: string) => Promise<void>;

    toggleWebSearch: () => void;
    sendMessage: (content: string) => Promise<void>;
    editAndResend: (messageId: string, newContent: string) => Promise<void>;
    switchBranch: (messageId: string) => Promise<void>;
    stopGeneration: () => Promise<void>;
    setTtsPlayingMessageId: (id: string | null) => void;
    speakMessage: (messageId: string, content: string) => Promise<void>;
    stopTts: () => Promise<void>;

    comparisons: Comparison[];
    activeComparisonId: string | null;
    comparisonMessages: ComparisonMessage[];
    compareStreamingLeft: boolean;
    compareStreamingRight: boolean;
    compareToolCallsLeft: ToolCallInfo[];
    compareToolCallsRight: ToolCallInfo[];

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

    loadSettings: () => Promise<void>;
    saveSettings: (settings: AppSettings) => Promise<void>;

    setActiveMode: (mode: string) => void;
    loadModeSettings: () => Promise<void>;
    updateModeSetting: (mode: string, enabled: boolean, sortOrder: number, config: string) => Promise<void>;

    // Plans
    activePlan: PlanWithTasks | null;
    planLoading: boolean;
    planGenerating: boolean;
    showPlanPanel: boolean;
    allPlans: AgentPlan[];
    generatePlan: (chatId: string, goal: string) => Promise<void>;
    loadPlanForChat: (chatId: string) => Promise<void>;
    loadAllPlans: () => Promise<void>;
    approvePlan: (planId: string, executionMode: string) => Promise<void>;
    startPlanExecution: (planId: string) => Promise<void>;
    executeSingleTask: (taskId: string) => Promise<string>;
    updatePlanTask: (taskId: string, updates: { title?: string; description?: string; status?: string }) => Promise<void>;
    deletePlan: (planId: string) => Promise<void>;
    setShowPlanPanel: (show: boolean) => void;
    initPlanListeners: () => Promise<void>;
    initWorkspaceListeners: () => Promise<void>;
    initSubAgentListeners: () => Promise<void>;
}


export const useChatStore = create<ChatState>((set, get) => ({
    ...createChatSlice(set, get),
    ...createSnippetsSlice(set, get),
    ...createSettingsSlice(set, get),
    ...createUiSlice(set, get),
    ...createAgentSlice(set, get),
}));

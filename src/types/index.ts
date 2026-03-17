// Вложение (черновик на фронте перед отправкой)
export interface Attachment {
    id: string;
    name: string;
    mimeType: string;
    size: number;
    previewUrl: string;
    data: number[];
}

export interface WebSource {
    title: string;
    url: string;
    snippet: string;
}

// Одно сообщение в чате
export interface Message {
    id: string;
    role: "system" | "user" | "assistant" | "tool";
    content: string;
    parentId?: string;
    timestamp: number;
    model?: string;
    promptTokens?: number;
    completionTokens?: number;
    cost?: number;
    hasAttachments?: boolean;
    webSources?: WebSource[];
    agentStep?: number;
    agentRunId?: string;
    ragSources?: RagSource[];
}

// Превью соседней ветки
export interface SiblingPreview {
    id: string;
    contentPreview: string;
    createdAt: number;
}

// Сообщение с информацией о ветках
export interface MessageWithSiblings extends Message {
    siblingCount: number;
    siblingIndex: number;
    siblingIds: string[];
    siblings: SiblingPreview[];
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
    mode?: string;
}

// Проект (группировка чатов в Assistant mode)
export interface Project {
    id: string;
    name: string;
    goal: string;
    status: 'active' | 'completed' | 'archived';
    createdAt: number;
    updatedAt: number;
}

export interface ProjectSummary extends Project {
    chatCount: number;
    artifactCount: number;
    memoryCount: number;
    planCount: number;
}

// Один чат (беседа)
export interface Chat {
    id: string;
    title: string;
    messages: MessageWithSiblings[];
    createdAt: number;
    updatedAt?: number;
    systemPrompt?: string;
    providerId?: string;
    model?: string;
    folderId?: string | null;
    projectId?: string | null;
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
    autoSkillDetection?: boolean;
}

export interface ImageStyle {
    id: string;
    name: string;
    promptSuffix: string;
    isBuiltin: boolean;
    sortOrder: number;
    createdAt: number;
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
    supportsToolUse?: boolean;
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
    sttProvider?: string;
    sttLanguage?: string;
    openaiApiKey?: string;
    groqSttApiKey?: string;
    ttsProvider?: "system" | "openai";
    ttsVoice?: string;
    ttsModel?: string;
    messageDensity?: "compact" | "standard" | "spacious";
    chatWidth?: "narrow" | "standard" | "wide";
    showStatusBar?: boolean;
    statusBarMetrics?: string[];
    webSearchProvider?: string;
    tavilyApiKey?: string;
    braveApiKey?: string;
    terminalFontSize?: number;
    terminalShell?: string;
    proxyEnabled?: boolean;
    proxyType?: string;
    proxyHost?: string;
    proxyPort?: number;
    proxyUsername?: string;
    proxyPassword?: string;
    sshHost?: string;
    sshPort?: number;
    sshUsername?: string;
    sshAuthType?: string;
    sshPassword?: string;
    sshKeyPath?: string;
    sshAutoConnect?: boolean;
    routingEnabled?: boolean;
    routingStrategy?: "rules" | "llm";
    budgetPlanEnabled?: boolean;
    budgetPlanLimit?: number;
    budgetGlobalEnabled?: boolean;
    budgetGlobalLimit?: number;
    budgetGlobalPeriod?: "daily" | "monthly";
    modelCatalogLastSync?: number;
    telegramBotToken?: string;
    telegramEnabled?: boolean;
    telegramAutoStart?: boolean;
    telegramModel?: string;
    embeddingOpenaiKey?: string;
    embeddingGeminiKey?: string;
}

export interface TelegramStatus {
    running: boolean;
    botUsername: string | null;
    authorizedUser: string | null;
}

export interface TelegramAuthRequest {
    userId: number;
    firstName: string;
    lastName: string | null;
    username: string | null;
}

export interface TelegramBotInfo {
    username: string;
    firstName: string;
}

// Payload событий стриминга — приходят из Rust через emit
export interface StreamPayload {
    content: string;
}

export interface StreamDonePayload {
    full_content: string;
    ragSources?: string | null;
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

export interface ToolCallEvent {
    toolCallId: string;
    serverId: string;
    toolName: string;
    arguments: string;
}

export interface ToolResultEvent {
    toolCallId: string;
    result: string;
    isError: boolean;
}

export interface ToolCallInfo {
    toolCallId: string;
    serverId: string;
    toolName: string;
    arguments: string;
    result?: string;
    isError?: boolean;
    status: "calling" | "done" | "error";
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
    showOnWelcome?: boolean;
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

export interface ChatTemplate {
    id: string;
    name: string;
    icon: string;
    color?: string;
    providerId: string;
    model: string;
    systemPrompt: string;
    temperature?: number | null;
    maxTokens?: number | null;
    topP?: number | null;
    topK?: number | null;
    frequencyPenalty?: number | null;
    presencePenalty?: number | null;
    sortOrder: number;
    createdAt: number;
}

export interface McpServer {
    id: string;
    name: string;
    command: string;
    args: string[];
    env: Record<string, string>;
    enabled: boolean;
    createdAt: number;
    serverType?: string;
    config?: string;
}

export interface FsMcpConfig {
    allowedDirectories: string[];
    blockedPatterns: string[];
    readOnly: boolean;
    maxFileSizeBytes: number;
    confirmDestructive: boolean;
}

export interface FsAuditEntry {
    id: number;
    timestamp: number;
    toolName: string;
    path: string;
    result: string;
    details: string;
    chatId: string;
}

export interface McpConnectionInfo {
    id: string;
    name: string;
    toolCount: number;
    connected: boolean;
}

export interface McpToolInfo {
    serverId: string;
    serverName: string;
    tool: {
        name: string;
        description: string | null;
        inputSchema: unknown;
    };
}

export interface Comparison {
    id: string;
    title: string;
    leftProviderId: string;
    leftModel: string;
    leftSystemPrompt?: string;
    rightProviderId: string;
    rightModel: string;
    rightSystemPrompt?: string;
    createdAt: number;
    updatedAt: number;
}

export interface ComparisonMessage {
    id: string;
    comparisonId: string;
    role: "user" | "assistant";
    side: "left" | "right" | null;
    content: string;
    timestamp: number;
    model?: string;
    promptTokens: number;
    completionTokens: number;
    cost: number;
    hasAttachments?: number;
}

export interface ComparisonStreamPayload {
    side: "left" | "right";
    content: string;
}

export interface ComparisonToolCallPayload {
    side: "left" | "right";
    toolCallId: string;
    serverId: string;
    toolName: string;
    arguments: string;
}

export interface ComparisonToolResultPayload {
    side: "left" | "right";
    toolCallId: string;
    result: string;
    isError: boolean;
}

export interface ComparisonStreamDonePayload {
    side: "left" | "right";
    full_content: string;
}

export interface ComparisonStreamUsagePayload {
    side: "left" | "right";
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
}

export interface ComparisonStreamErrorPayload {
    side: "left" | "right";
    error: string;
}

export interface ComparisonStreamImagePayload {
    side: "left" | "right";
    messageId: string;
    path: string;
    index: number;
}

export interface StreamRetryPayload {
    attempt: number;
    maxAttempts: number;
    delayMs: number;
}

export interface ComparisonStreamRetryPayload {
    side: "left" | "right";
    attempt: number;
    maxAttempts: number;
    delayMs: number;
}

export interface PromptLibraryItem {
    id: string;
    title: string;
    description: string;
    content: string;
    category: string;
    isBuiltin: boolean;
    language: string;
    createdAt: number;
    updatedAt: number;
}

export interface ModeSettingRow {
    mode: string;
    enabled: boolean;
    sortOrder: number;
    config: string;
}

export interface ModeWorkspace {
    activeChatId: string | null;
    activeComparisonId: string | null;
}

export type ModeStateMap = Record<string, ModeWorkspace>;

// Model catalog and routing
export interface ModelCatalogEntry {
    id: string;
    provider: string;
    modelId: string;
    displayName: string;
    costPerInputToken?: number;
    costPerOutputToken?: number;
    contextWindow?: number;
    category: string;
    strengths?: string;
    isAvailable: boolean;
    updatedAt: number;
}

export interface RoutingRule {
    id: number;
    taskCategory: string;
    preferredModel: string;
    fallbackModel?: string;
    priority: number;
    enabled: boolean;
}

// Agent types
export interface AgentRun {
    id: string;
    chatId: string;
    status: "running" | "paused" | "completed" | "failed" | "cancelled" | "paused_budget";
    iterations: number;
    maxIterations: number;
    startedAt: number;
    finishedAt?: number;
    error?: string;
    cost?: number;
    assignedModel?: string;
}

export interface AgentRunStartedPayload {
    runId: string;
    chatId: string;
    maxIterations: number;
}

export interface AgentStepPayload {
    runId: string;
    chatId: string;
    iteration: number;
}

export interface AgentRunFinishedPayload {
    runId: string;
    status: string;
    iterations: number;
    error?: string;
}

export interface AgentRunLimitPayload {
    runId: string;
    iteration: number;
    maxIterations: number;
}

export type AgentStatus = "idle" | "running" | "paused" | "completed" | "failed" | "cancelled";

// Agent Memory types
export interface AgentMemory {
    id: string;
    content: string;
    category: string;
    sourceChatId?: string;
    sourceMessageId?: string;
    projectId?: string | null;
    isPinned: boolean;
    createdAt: number;
    updatedAt: number;
}

export interface AgentMemorySearchResult {
    memory: AgentMemory;
    score: number;
}

export type MemoryCategory = "fact" | "decision" | "preference" | "context" | "learning";

// Skills
export interface Skill {
    id: string;
    name: string;
    description: string;
    icon: string;
    content: string;
    triggerDescription: string;
    requiredTools: string;
    isBuiltin: boolean;
    enabled: boolean;
    sortOrder: number;
    createdAt: number;
    updatedAt: number;
}

export interface ChatSkill {
    chatId: string;
    skillId: string;
    attachedBy: string;
    createdAt: number;
}

// ─── Plans ──────────────────────────────────────────────────

export interface AgentPlan {
    id: string;
    chatId: string;
    projectId?: string | null;
    goal: string;
    status: string;
    executionMode: string;
    replanCount: number;
    createdAt: number;
    updatedAt: number;
}

export interface AgentTask {
    id: string;
    planId: string;
    title: string;
    description: string;
    status: string;
    dependencies: string[];
    result: string | null;
    agentRunId: string | null;
    sortOrder: number;
    createdAt: number;
    updatedAt: number;
    category?: string;
    assignedModel?: string;
}

export interface PlanWithTasks {
    plan: AgentPlan;
    tasks: AgentTask[];
}

export interface AgentPlanWithProgress extends AgentPlan {
    totalTasks: number;
    completedTasks: number;
}

// ─── Workspace ─────────────────────────────────────────────

export interface WorkspaceArtifact {
    id: string;
    chatId: string;
    projectId: string | null;
    name: string;
    contentType: "code" | "text" | "data" | "image";
    content: string | null;
    filePath: string | null;
    createdBy: string | null;
    updatedBy: string | null;
    createdAt: number;
    updatedAt: number;
}

// ─── Sub-Agents ────────────────────────────────────────────

export interface SubAgentInfo {
    agentRunId: string;
    parentRunId: string;
    goal: string;
    model: string | null;
    status: "running" | "completed" | "failed" | "cancelled";
    iterations: number;
    maxIterations: number;
    cost: number | null;
}

export interface SubAgentRunInfo {
    id: string;
    parentRunId: string | null;
    status: string;
    goal: string;
    iterations: number;
    maxIterations: number;
    cost: number | null;
    assignedModel: string | null;
}

// ─── Scheduler ────────────────────────────────────────────

export interface ScheduledTask {
    id: string;
    name: string;
    prompt: string;
    cronExpression: string;
    enabled: boolean;
    mode: string;
    model: string | null;
    skillId: string | null;
    projectId: string | null;
    deliverTelegram: boolean;
    deliverDesktopNotification: boolean;
    lastRunAt: number | null;
    nextRunAt: number | null;
    lastRunStatus: string | null;
    lastRunError: string | null;
    runCount: number;
    createdAt: number;
    updatedAt: number;
}

export interface SchedulerStatus {
    running: boolean;
    taskCount: number;
    nextTaskAt: number | null;
}

export interface KnowledgeBase {
    id: string;
    name: string;
    description: string;
    embeddingModel: string;
    embeddingDimensions: number;
    chunkingStrategy: string;
    chunkSize: number;
    chunkOverlap: number;
    retrievalTopK: number;
    retrievalMinScore: number;
    systemPrompt: string;
    version: number;
    status: string;
    documentCount: number;
    totalChunks: number;
    createdAt: number;
    updatedAt: number;
}

export interface KbDocument {
    id: string;
    kbId: string;
    name: string;
    sourceType: string;
    sourcePath: string | null;
    sourceUrl: string | null;
    mimeType: string;
    fileSize: number;
    chunkCount: number;
    indexingStatus: string;
    indexingError: string | null;
    contentHash: string | null;
    createdAt: number;
    updatedAt: number;
}

export interface KbStats {
    documentCount: number;
    totalChunks: number;
    indexedDocuments: number;
    pendingDocuments: number;
    failedDocuments: number;
    totalFileSize: number;
}

export interface KbSearchResultItem {
    chunkId: string;
    documentId: string;
    documentName: string;
    content: string;
    chunkIndex: number;
    score: number;
    metadata: Record<string, unknown>;
}

export interface RagSource {
    documentId: string;
    documentName: string;
    chunkIndex: number;
    content: string;
    score: number;
}
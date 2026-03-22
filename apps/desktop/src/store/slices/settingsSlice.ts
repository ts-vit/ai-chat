import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import i18n from "../../i18n";
import { notify } from "../../utils/notify";
import type {
    AppSettings,
    BalanceInfo,
    CustomProvider,
    FsAuditEntry,
    FsMcpConfig,
    ImageStyle,
    McpConnectionInfo,
    McpServer,
    ModelInfo,
    OllamaLocalModel,
} from "../../types";
import type { ChatState } from "../chatStore";

export interface SettingsSlice {
    // State
    settings: AppSettings;
    customProviders: CustomProvider[];
    models: ModelInfo[];
    modelsLoading: boolean;
    modelsError: string | null;
    ollamaStatus: "unknown" | "available" | "unavailable";
    localOllamaModels: OllamaLocalModel[];
    ollamaPullProgress: { model: string; progress: number; status: string } | null;
    balance: BalanceInfo | null;
    mcpServers: McpServer[];
    mcpConnections: McpConnectionInfo[];
    fsConfig: FsMcpConfig | null;
    fsAuditLog: FsAuditEntry[];
    imageStyles: ImageStyle[];

    // Actions — Settings
    loadSettings: () => Promise<void>;
    saveSettings: (settings: AppSettings) => Promise<void>;

    // Actions — Custom Providers
    loadCustomProviders: () => Promise<void>;
    createCustomProvider: (name: string, baseUrl: string, apiKey: string) => Promise<void>;
    updateCustomProvider: (id: string, name: string, baseUrl: string, apiKey: string) => Promise<void>;
    deleteCustomProvider: (id: string) => Promise<void>;

    // Actions — Ollama
    testOllamaConnection: (baseUrl: string) => Promise<number>;
    checkOllamaStatus: () => Promise<void>;
    loadLocalOllamaModels: () => Promise<void>;
    deleteOllamaModel: (modelName: string) => Promise<void>;
    pullOllamaModel: (modelName: string) => Promise<void>;
    clearOllamaPullProgress: () => void;
    initOllamaPullListeners: () => Promise<void>;

    // Actions — Models & Balance
    loadModels: (force?: boolean) => Promise<void>;
    loadBalance: () => Promise<void>;

    // Actions — MCP
    loadMcpServers: () => Promise<void>;
    loadMcpConnections: () => Promise<void>;
    addMcpServer: (name: string, command: string, args: string[], env: Record<string, string>) => Promise<void>;
    updateMcpServer: (id: string, name: string, command: string, args: string[], env: Record<string, string>, enabled: boolean) => Promise<void>;
    removeMcpServer: (id: string) => Promise<void>;
    toggleMcpServer: (id: string, enabled: boolean) => Promise<void>;
    connectMcpServer: (id: string) => Promise<void>;

    // Actions — Filesystem
    loadFsConfig: () => Promise<void>;
    saveFsConfig: (config: FsMcpConfig) => Promise<void>;
    loadFsAuditLog: (limit?: number) => Promise<void>;
    clearFsAuditLog: () => Promise<void>;

    // Actions — Image Styles
    loadImageStyles: () => Promise<void>;
    createImageStyle: (name: string, promptSuffix: string) => Promise<void>;
    updateImageStyle: (id: string, name: string, promptSuffix: string) => Promise<void>;
    deleteImageStyle: (id: string) => Promise<void>;

    // Actions — TTS Listeners
    initTtsListeners: () => Promise<void>;
}

type Set = (partial: Partial<ChatState> | ((state: ChatState) => Partial<ChatState>)) => void;
type Get = () => ChatState;

export const createSettingsSlice = (set: Set, get: Get): SettingsSlice => ({
    // ── State ──────────────────────────────────────────────

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
        messageDensity: "standard",
        chatWidth: "standard",
        showStatusBar: true,
        statusBarMetrics: ["balance", "context", "tokens", "cost"],
    },
    customProviders: [],
    models: [],
    modelsLoading: false,
    modelsError: null,
    ollamaStatus: "unknown",
    localOllamaModels: [],
    ollamaPullProgress: null,
    balance: null,
    mcpServers: [],
    mcpConnections: [],
    fsConfig: null,
    fsAuditLog: [],
    imageStyles: [],

    // ── Settings ──────────────────────────────────────────

    loadSettings: async () => {
        try {
            const all = await invoke<Array<{ key: string; value: string; is_sensitive: boolean }>>(
                "get_all_settings",
                { prefix: "" }
            );
            const map: Record<string, string> = {};
            for (const s of all) {
                map[s.key] = s.value;
            }

            const bool = (k: string, def: boolean) =>
                k in map ? map[k] === "true" : def;
            const str = (k: string, def = "") =>
                map[k] ?? def;
            const num = (k: string, def: number) =>
                k in map ? Number(map[k]) : def;
            const optStr = (k: string) =>
                map[k] && map[k].trim() ? map[k] : undefined;
            const jsonArr = (k: string): string[] => {
                try { return map[k] ? JSON.parse(map[k]) : []; }
                catch { return []; }
            };
            const jsonObj = (k: string): Record<string, string[]> => {
                try { return map[k] ? JSON.parse(map[k]) : {}; }
                catch { return {}; }
            };

            const settings: AppSettings = {
                api_key:                    str("llm.openrouter.api_key"),
                management_key:             str("llm.openrouter.mgmt_key"),
                model:                      str("llm.openrouter.model", "anthropic/claude-sonnet-4-20250514"),
                temperature:                num("llm.temperature", 0.7),
                max_tokens:                 num("llm.max_tokens", 4096),
                font_size:                  num("ui.font_size", 14),
                ollamaUrl:                  str("llm.ollama.url", "http://localhost:11434/v1"),
                openrouterEnabledModels:    jsonArr("llm.openrouter.models"),
                ollamaEnabledModels:        jsonArr("llm.ollama.models"),
                customProviderEnabledModels: jsonObj("llm.custom.models"),
                language:                   str("ui.language"),
                sendByEnter:                bool("ui.send_by_enter", true),
                topP:                       map["llm.top_p"] ? Number(map["llm.top_p"]) : undefined,
                topK:                       map["llm.top_k"] ? Number(map["llm.top_k"]) : undefined,
                frequencyPenalty:           map["llm.frequency_penalty"] ? Number(map["llm.frequency_penalty"]) : undefined,
                presencePenalty:            map["llm.presence_penalty"] ? Number(map["llm.presence_penalty"]) : undefined,
                sttProvider:                optStr("audio.stt.provider"),
                sttLanguage:                optStr("audio.stt.language"),
                openaiApiKey:               optStr("audio.stt.openai.api_key"),
                groqSttApiKey:              optStr("audio.stt.groq.api_key"),
                ttsProvider:                optStr("audio.tts.provider") as AppSettings["ttsProvider"],
                ttsVoice:                   optStr("audio.tts.voice"),
                ttsModel:                   optStr("audio.tts.model"),
                messageDensity:             (str("ui.message_density", "standard")) as AppSettings["messageDensity"],
                chatWidth:                  (str("ui.chat_width", "standard")) as AppSettings["chatWidth"],
                showStatusBar:              bool("ui.show_status_bar", true),
                statusBarMetrics:           jsonArr("ui.status_bar_metrics").length > 0
                                                ? jsonArr("ui.status_bar_metrics")
                                                : ["balance", "context", "tokens", "cost"],
                webSearchProvider:          optStr("search.provider"),
                tavilyApiKey:               optStr("search.tavily.api_key"),
                braveApiKey:                optStr("search.brave.api_key"),
                terminalFontSize:           map["terminal.font_size"] ? Number(map["terminal.font_size"]) : undefined,
                terminalShell:              optStr("terminal.shell"),
                sshHost:                    optStr("ssh.host"),
                sshPort:                    map["ssh.port"] ? Number(map["ssh.port"]) : undefined,
                sshUsername:                optStr("ssh.username"),
                sshAuthType:                optStr("ssh.auth_type"),
                sshPassword:                optStr("ssh.password"),
                sshKeyPath:                 optStr("ssh.key_path"),
                sshAutoConnect:             bool("ssh.auto_connect", false),
                routingEnabled:             bool("routing.enabled", false),
                routingStrategy:            optStr("routing.strategy") as AppSettings["routingStrategy"],
                budgetPlanEnabled:          bool("budget.plan.enabled", false),
                budgetPlanLimit:            map["budget.plan.limit"] ? Number(map["budget.plan.limit"]) : undefined,
                budgetGlobalEnabled:        bool("budget.global.enabled", false),
                budgetGlobalLimit:          map["budget.global.limit"] ? Number(map["budget.global.limit"]) : undefined,
                budgetGlobalPeriod:         optStr("budget.global.period") as AppSettings["budgetGlobalPeriod"],
                modelCatalogLastSync:       map["model.catalog.last_sync"] ? Number(map["model.catalog.last_sync"]) : undefined,
                telegramBotToken:           optStr("telegram.bot_token"),
                telegramEnabled:            bool("telegram.enabled", false),
                telegramAutoStart:          bool("telegram.auto_start", false),
                telegramModel:              optStr("telegram.model"),
                embeddingOpenaiKey:         optStr("embedding.openai.api_key"),
                embeddingGeminiKey:         optStr("embedding.gemini.api_key"),
                rerankerCohereKey:          optStr("reranker.cohere.api_key"),
                rerankerJinaKey:            optStr("reranker.jina.api_key"),
            };

            set({ settings });
            const lang = settings.language?.trim() ||
                (navigator.language.startsWith("ru") ? "ru" : "en");
            i18n.changeLanguage(lang);
            get().loadImageStyles();
        } catch (e) {
            notify.error(i18n.t("notifications.settingsLoadError"));
        }
    },

    saveSettings: async (settings: AppSettings) => {
        try {
            const s = (key: string, value: string | undefined | null) => {
                if (value !== undefined && value !== null) {
                    return invoke("set_setting", { key, value: String(value) });
                }
                return invoke("delete_setting", { key });
            };

            await Promise.all([
                s("llm.openrouter.model",            settings.model),
                s("llm.custom.models",               JSON.stringify(settings.customProviderEnabledModels)),
                s("audio.stt.provider",              settings.sttProvider),
                s("audio.stt.language",              settings.sttLanguage),
                s("audio.stt.openai.api_key",        settings.openaiApiKey),
                s("audio.stt.groq.api_key",          settings.groqSttApiKey),
                s("audio.tts.provider",              settings.ttsProvider),
                s("audio.tts.voice",                 settings.ttsVoice),
                s("audio.tts.model",                 settings.ttsModel),
                s("ssh.host",                        settings.sshHost),
                s("ssh.port",                        settings.sshPort != null ? String(settings.sshPort) : null),
                s("ssh.username",                    settings.sshUsername),
                s("ssh.auth_type",                   settings.sshAuthType),
                s("ssh.password",                    settings.sshPassword),
                s("ssh.key_path",                    settings.sshKeyPath),
                s("ssh.auto_connect",                String(settings.sshAutoConnect ?? false)),
                s("routing.enabled",                 String(settings.routingEnabled ?? false)),
                s("routing.strategy",                settings.routingStrategy),
                s("model.catalog.last_sync",         settings.modelCatalogLastSync != null ? String(settings.modelCatalogLastSync) : null),
                s("telegram.bot_token",              settings.telegramBotToken),
                s("telegram.enabled",                String(settings.telegramEnabled ?? false)),
                s("telegram.auto_start",             String(settings.telegramAutoStart ?? false)),
                s("telegram.model",                  settings.telegramModel),
                s("embedding.openai.api_key",        settings.embeddingOpenaiKey),
                s("embedding.gemini.api_key",        settings.embeddingGeminiKey),
                s("reranker.cohere.api_key",         settings.rerankerCohereKey),
                s("reranker.jina.api_key",           settings.rerankerJinaKey),
            ]);

            set({ settings: { ...get().settings, ...settings } });
            notify.success(i18n.t("notifications.settingsSaved"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Custom Providers ──────────────────────────────────

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
            await invoke("create_custom_provider", { input: { name, baseUrl, apiKey } });
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

    // ── Ollama ────────────────────────────────────────────

    testOllamaConnection: async (baseUrl) => {
        const list = await invoke<unknown[]>("get_ollama_models", { baseUrl });
        return Array.isArray(list) ? list.length : 0;
    },

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

    pullOllamaModel: async (modelName) => {
        set({
            ollamaPullProgress: {
                model: modelName,
                progress: 0,
                status: "",
            },
        });
        try {
            await invoke("pull_ollama_model", { modelName });
        } catch (e) {
            set({ ollamaPullProgress: null });
            notify.error(String(e));
        }
    },

    clearOllamaPullProgress: () => set({ ollamaPullProgress: null }),

    initOllamaPullListeners: async () => {
        await listen<{
            status?: string;
            completed?: number;
            total?: number;
        }>("ollama-pull-progress", (event) => {
            const { status, completed, total } = event.payload;
            set((state) => {
                if (!state.ollamaPullProgress) return state;
                const progress =
                    typeof total === "number" && total > 0 && typeof completed === "number"
                        ? Math.round((completed / total) * 100)
                        : state.ollamaPullProgress.progress;
                return {
                    ollamaPullProgress: {
                        ...state.ollamaPullProgress,
                        status: status ?? state.ollamaPullProgress.status,
                        progress,
                    },
                };
            });
        });
        await listen<{ model: string }>("ollama-pull-done", () => {
            set({ ollamaPullProgress: null });
            get().loadLocalOllamaModels();
            notify.success(i18n.t("notifications.modelDownloaded"));
        });
        await listen<{ message: string }>("ollama-pull-error", (event) => {
            set({ ollamaPullProgress: null });
            const msg = event.payload?.message?.trim();
            notify.error(msg ? `${i18n.t("notifications.error")}: ${msg}` : i18n.t("notifications.error"));
        });
    },

    // ── Models & Balance ──────────────────────────────────

    loadModels: async (force) => {
        const { models } = get();
        if (!force && models.length > 0) return;
        set({ modelsLoading: true, modelsError: null, ...(force ? { models: [] } : {}) });
        try {
            const res = await fetch("https://openrouter.ai/api/v1/models");
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const response = await res.json();
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
                supported_parameters?: string[];
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
                        supportsToolUse: Array.isArray(m.supported_parameters)
                            ? m.supported_parameters.includes("tools")
                            : true,
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

    // ── MCP ───────────────────────────────────────────────

    loadMcpServers: async () => {
        try {
            const list = await invoke<McpServer[]>("mcp_get_servers");
            set({ mcpServers: list ?? [] });
        } catch (e) {
            console.error("Failed to load MCP servers:", e);
        }
    },

    loadMcpConnections: async () => {
        try {
            const list = await invoke<McpConnectionInfo[]>("mcp_list_connections");
            set({ mcpConnections: list ?? [] });
        } catch (e) {
            console.error("Failed to load MCP connections:", e);
        }
    },

    addMcpServer: async (name, command, args, env) => {
        try {
            await invoke("mcp_add_server", { name, command, args, env, enabled: true });
            await get().loadMcpServers();
            await get().loadMcpConnections();
            notify.success(i18n.t("notifications.mcpServerAdded"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateMcpServer: async (id, name, command, args, env, enabled) => {
        try {
            await invoke("mcp_update_server", { id, name, command, args, env, enabled });
            await get().loadMcpServers();
            await get().loadMcpConnections();
            notify.success(i18n.t("notifications.mcpServerUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    removeMcpServer: async (id) => {
        try {
            await invoke("mcp_remove_server", { id });
            await get().loadMcpServers();
            await get().loadMcpConnections();
            notify.success(i18n.t("notifications.mcpServerRemoved"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    toggleMcpServer: async (id, enabled) => {
        try {
            await invoke("mcp_toggle_server", { id, enabled });
            await get().loadMcpServers();
            await get().loadMcpConnections();
        } catch (e) {
            notify.error(String(e));
        }
    },

    connectMcpServer: async (id) => {
        try {
            await invoke("mcp_connect_by_id", { serverId: id });
            await get().loadMcpConnections();
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Filesystem ────────────────────────────────────────

    loadFsConfig: async () => {
        try {
            const json = await invoke<string>("get_fs_mcp_config");
            const config: FsMcpConfig = JSON.parse(json);
            set({ fsConfig: config });
        } catch (e) {
            console.error("Failed to load FS config:", e);
        }
    },

    saveFsConfig: async (config) => {
        try {
            await invoke("update_fs_mcp_config", { config: JSON.stringify(config) });
            set({ fsConfig: config });
            notify.success(i18n.t("notifications.saved"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    loadFsAuditLog: async (limit = 100) => {
        try {
            const log = await invoke<FsAuditEntry[]>("get_fs_audit_log", { limit, offset: 0 });
            set({ fsAuditLog: log ?? [] });
        } catch (e) {
            console.error("Failed to load FS audit log:", e);
        }
    },

    clearFsAuditLog: async () => {
        try {
            await invoke("clear_fs_audit_log");
            set({ fsAuditLog: [] });
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Image Styles ──────────────────────────────────────

    loadImageStyles: async () => {
        try {
            const styles = await invoke<ImageStyle[]>("get_image_styles");
            set({ imageStyles: styles ?? [] });
        } catch (e) {
            console.error("Failed to load image styles:", e);
        }
    },

    createImageStyle: async (name, promptSuffix) => {
        try {
            await invoke("create_image_style", { name, promptSuffix });
            await get().loadImageStyles();
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateImageStyle: async (id, name, promptSuffix) => {
        try {
            await invoke("update_image_style", { id, name, promptSuffix });
            await get().loadImageStyles();
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteImageStyle: async (id) => {
        try {
            await invoke("delete_image_style", { id });
            await get().loadImageStyles();
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── TTS Listeners ─────────────────────────────────────

    initTtsListeners: async () => {
        await listen("tts-playback-done", () => {
            set({ playingMessageId: null });
        });
    },
});

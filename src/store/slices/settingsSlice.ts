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
                messageDensity: loaded.messageDensity ?? "standard",
                chatWidth: loaded.chatWidth ?? "standard",
                showStatusBar: loaded.showStatusBar ?? true,
                statusBarMetrics: Array.isArray(loaded.statusBarMetrics) && loaded.statusBarMetrics.length > 0
                    ? loaded.statusBarMetrics
                    : ["balance", "context", "tokens", "cost"],
            };
            set({ settings });
            const lang = settings.language?.trim() || (navigator.language.startsWith("ru") ? "ru" : "en");
            i18n.changeLanguage(lang);
            get().loadImageStyles();
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

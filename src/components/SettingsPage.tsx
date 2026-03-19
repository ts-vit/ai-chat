import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { AppSettings } from "../types";
import {
    Box,
    Button,
    Group,
    NavLink,
    PasswordInput,
    ScrollArea,
    Stack,
    Text,
    Title,
} from "@mantine/core";
import {
    IconBox,
    IconCpu,
    IconDatabase,
    IconFileText,
    IconFolder,
    IconInfoCircle,
    IconKey,
    IconLayoutGrid,
    IconMicrophone,
    IconPalette,
    IconPlug,
    IconRoute,
    IconSettings,
    IconNetwork,
    IconTerminal2,
    IconWorldSearch,
    IconBrain,
    IconApps,
    IconBrandTelegram,
    IconVectorTriangle,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { notify } from "../utils/notify";
import {
    AboutSection,
    AudioSection,
    BudgetSection,
    CustomProvidersSection,
    DataSection,
    GenerationSection,
    InterfaceSection,
    FilesystemSection,
    McpSection,
    OllamaModelsSection,
    OpenRouterSection,
    PresetsSection,
    RoutingSection,
    VpnSection,
    TemplatesSection,
    TerminalSection,
    WebSearchSection,
    SearchModelSection,
    ModesSection,
    TelegramSection,
} from "./settings";

export type SettingsSection =
    | "openRouter"
    | "ollama"
    | "customProviders"
    | "generation"
    | "interface"
    | "modes"
    | "routing"
    | "presets"
    | "templates"
    | "mcp"
    | "filesystem"
    | "webSearch"
    | "proxy"
    | "audio"
    | "terminal"
    | "searchModel"
    | "telegram"
    | "embedding"
    | "data"
    | "about";

const getNavItems = (t: (key: string) => string): { section: SettingsSection; label: string; icon: React.ReactNode }[] => [
    { section: "openRouter", label: t("settings.nav.openRouter"), icon: <IconKey size={18} stroke={1.5} /> },
    { section: "ollama", label: t("settings.nav.ollama"), icon: <IconCpu size={18} stroke={1.5} /> },
    { section: "customProviders", label: t("settings.nav.customProviders"), icon: <IconBox size={18} stroke={1.5} /> },
    { section: "generation", label: t("settings.nav.generation"), icon: <IconSettings size={18} stroke={1.5} /> },
    { section: "interface", label: t("settings.nav.interface"), icon: <IconPalette size={18} stroke={1.5} /> },
    { section: "modes", label: t("settings.nav.modes"), icon: <IconApps size={18} stroke={1.5} /> },
    { section: "routing", label: t("routing.title"), icon: <IconRoute size={18} stroke={1.5} /> },
    { section: "presets", label: t("settings.nav.presets"), icon: <IconFileText size={18} stroke={1.5} /> },
    { section: "templates", label: t("settings.nav.templates"), icon: <IconLayoutGrid size={18} stroke={1.5} /> },
    { section: "mcp", label: t("settings.nav.mcp"), icon: <IconPlug size={18} stroke={1.5} /> },
    { section: "filesystem", label: t("settings.nav.filesystem"), icon: <IconFolder size={18} stroke={1.5} /> },
    { section: "webSearch", label: t("settings.nav.webSearch"), icon: <IconWorldSearch size={18} stroke={1.5} /> },
    { section: "proxy", label: t("settings.nav.vpn"), icon: <IconNetwork size={18} stroke={1.5} /> },
    { section: "audio", label: t("settings.nav.audio"), icon: <IconMicrophone size={18} stroke={1.5} /> },
    { section: "telegram", label: t("telegram.title"), icon: <IconBrandTelegram size={18} stroke={1.5} /> },
    { section: "terminal", label: t("settings.nav.terminal"), icon: <IconTerminal2 size={18} stroke={1.5} /> },
    { section: "embedding", label: t("settings.nav.embedding"), icon: <IconVectorTriangle size={18} stroke={1.5} /> },
    { section: "searchModel", label: t("settings.nav.searchModel"), icon: <IconBrain size={18} stroke={1.5} /> },
    { section: "data", label: t("settings.nav.data"), icon: <IconDatabase size={18} stroke={1.5} /> },
    { section: "about", label: t("settings.nav.about"), icon: <IconInfoCircle size={18} stroke={1.5} /> },
];

/** Snapshot of form fields that are saved via the main Save button (excludes model, stt*, openaiApiKey). */
interface FormSnapshot {
    api_key: string;
    management_key: string;
    temperature: number;
    max_tokens: number;
    font_size: number;
    ollamaUrl: string;
    openrouterEnabledModels: string[];
    ollamaEnabledModels: string[];
    customProviderEnabledModels: Record<string, string[]>;
    topP: number | null;
    topK: number | null;
    frequencyPenalty: number | null;
    presencePenalty: number | null;
    language: string;
    sendByEnter: boolean;
    messageDensity: string;
    chatWidth: string;
    showStatusBar: boolean;
    statusBarMetrics: string[];
    webSearchProvider: string;
    tavilyApiKey: string;
    braveApiKey: string;
    terminalFontSize: number;
    terminalShell: string;
    proxyEnabled: boolean;
    proxyType: string;
    proxyHost: string;
    proxyPort: number | undefined;
    proxyUsername: string;
    proxyPassword: string;
    sshHost: string;
    sshPort: number | undefined;
    sshUsername: string;
    sshAuthType: string;
    sshPassword: string;
    sshKeyPath: string;
    sshAutoConnect: boolean;
    telegramBotToken: string;
    telegramEnabled: boolean;
    telegramAutoStart: boolean;
    telegramModel: string;
    embeddingOpenaiKey: string;
    embeddingGeminiKey: string;
    rerankerCohereKey: string;
    rerankerJinaKey: string;
}

function normalizedCustomProviders(obj: Record<string, string[]>): Record<string, string[]> {
    return Object.fromEntries(
        Object.entries(obj).map(([k, v]) => [k, (v ?? []).slice(0, 5)])
    );
}

function isSameSnapshot(a: FormSnapshot, b: FormSnapshot | null): boolean {
    if (b === null) return false;
    if (
        a.api_key !== b.api_key ||
        a.management_key !== b.management_key ||
        a.temperature !== b.temperature ||
        a.max_tokens !== b.max_tokens ||
        a.font_size !== b.font_size ||
        a.ollamaUrl !== b.ollamaUrl ||
        a.language !== b.language ||
        a.sendByEnter !== b.sendByEnter ||
        a.messageDensity !== b.messageDensity ||
        a.chatWidth !== b.chatWidth ||
        a.showStatusBar !== b.showStatusBar ||
        a.topP !== b.topP ||
        a.topK !== b.topK ||
        a.frequencyPenalty !== b.frequencyPenalty ||
        a.presencePenalty !== b.presencePenalty ||
        a.webSearchProvider !== b.webSearchProvider ||
        a.tavilyApiKey !== b.tavilyApiKey ||
        a.braveApiKey !== b.braveApiKey ||
        a.terminalFontSize !== b.terminalFontSize ||
        a.terminalShell !== b.terminalShell ||
        a.proxyEnabled !== b.proxyEnabled ||
        a.proxyType !== b.proxyType ||
        a.proxyHost !== b.proxyHost ||
        a.proxyPort !== b.proxyPort ||
        a.proxyUsername !== b.proxyUsername ||
        a.proxyPassword !== b.proxyPassword ||
        a.sshHost !== b.sshHost ||
        a.sshPort !== b.sshPort ||
        a.sshUsername !== b.sshUsername ||
        a.sshAuthType !== b.sshAuthType ||
        a.sshPassword !== b.sshPassword ||
        a.sshKeyPath !== b.sshKeyPath ||
        a.sshAutoConnect !== b.sshAutoConnect ||
        a.telegramBotToken !== b.telegramBotToken ||
        a.telegramEnabled !== b.telegramEnabled ||
        a.telegramAutoStart !== b.telegramAutoStart ||
        a.telegramModel !== b.telegramModel ||
        a.embeddingOpenaiKey !== b.embeddingOpenaiKey ||
        a.embeddingGeminiKey !== b.embeddingGeminiKey ||
        a.rerankerCohereKey !== b.rerankerCohereKey ||
        a.rerankerJinaKey !== b.rerankerJinaKey
    ) {
        return false;
    }
    if (
        a.openrouterEnabledModels.length !== b.openrouterEnabledModels.length ||
        a.openrouterEnabledModels.some((v, i) => v !== b.openrouterEnabledModels[i])
    ) {
        return false;
    }
    if (
        a.ollamaEnabledModels.length !== b.ollamaEnabledModels.length ||
        a.ollamaEnabledModels.some((v, i) => v !== b.ollamaEnabledModels[i])
    ) {
        return false;
    }
    const aKeys = Object.keys(a.customProviderEnabledModels).sort();
    const bKeys = Object.keys(b.customProviderEnabledModels).sort();
    if (aKeys.length !== bKeys.length || aKeys.some((k, i) => k !== bKeys[i])) return false;
    for (const k of aKeys) {
        const av = a.customProviderEnabledModels[k] ?? [];
        const bv = b.customProviderEnabledModels[k] ?? [];
        if (av.length !== bv.length || av.some((v, i) => v !== bv[i])) return false;
    }
    if (
        a.statusBarMetrics.length !== b.statusBarMetrics.length ||
        a.statusBarMetrics.some((v, i) => v !== b.statusBarMetrics[i])
    ) {
        return false;
    }
    return true;
}

export function SettingsPage() {
    const { t } = useTranslation();
    const { settings, saveSettings, setView } = useChatStore();
    const NAV_ITEMS = getNavItems(t);

    const [activeSection, setActiveSection] = useState<SettingsSection>("openRouter");
    const [apiKey, setApiKey] = useState(settings.api_key);
    const [managementKey, setManagementKey] = useState(settings.management_key);
    const [ollamaUrl, setOllamaUrl] = useState(settings.ollamaUrl);
    const [temperature, setTemperature] = useState(settings.temperature);
    const [maxTokens, setMaxTokens] = useState(settings.max_tokens);
    const [fontSize, setFontSize] = useState(settings.font_size);
    const [openrouterEnabledModels, setOpenrouterEnabledModels] = useState<string[]>(
        settings.openrouterEnabledModels ?? []
    );
    const [ollamaEnabledModels, setOllamaEnabledModels] = useState<string[]>(
        settings.ollamaEnabledModels ?? []
    );
    const [customProviderEnabledModels, setCustomProviderEnabledModels] = useState<
        Record<string, string[]>
    >(settings.customProviderEnabledModels ?? {});

    const [topP, setTopP] = useState(settings.topP ?? 0.9);
    const [topPEnabled, setTopPEnabled] = useState(settings.topP != null);
    const [topK, setTopK] = useState(settings.topK ?? 40);
    const [topKEnabled, setTopKEnabled] = useState(settings.topK != null);
    const [frequencyPenalty, setFrequencyPenalty] = useState(
        settings.frequencyPenalty ?? 0
    );
    const [frequencyPenaltyEnabled, setFrequencyPenaltyEnabled] = useState(
        settings.frequencyPenalty != null
    );
    const [presencePenalty, setPresencePenalty] = useState(
        settings.presencePenalty ?? 0
    );
    const [presencePenaltyEnabled, setPresencePenaltyEnabled] = useState(
        settings.presencePenalty != null
    );
    const [language, setLanguage] = useState(settings.language ?? "");
    const [sendByEnter, setSendByEnter] = useState(settings.sendByEnter ?? true);
    const [messageDensity, setMessageDensity] = useState(settings.messageDensity ?? "standard");
    const [chatWidth, setChatWidth] = useState(settings.chatWidth ?? "standard");
    const [showStatusBar, setShowStatusBar] = useState(settings.showStatusBar ?? true);
    const [statusBarMetrics, setStatusBarMetrics] = useState<string[]>(
        settings.statusBarMetrics ?? ["balance", "context", "tokens", "cost"]
    );
    const [webSearchProvider, setWebSearchProvider] = useState(settings.webSearchProvider ?? "");
    const [tavilyApiKey, setTavilyApiKey] = useState(settings.tavilyApiKey ?? "");
    const [braveApiKey, setBraveApiKey] = useState(settings.braveApiKey ?? "");
    const [terminalFontSize, setTerminalFontSize] = useState(settings.terminalFontSize ?? 13);
    const [terminalShell, setTerminalShell] = useState(settings.terminalShell ?? "");
    const [proxyEnabled, setProxyEnabled] = useState(settings.proxyEnabled ?? false);
    const [proxyType, setProxyType] = useState(settings.proxyType ?? "http");
    const [proxyHost, setProxyHost] = useState(settings.proxyHost ?? "");
    const [proxyPort, setProxyPort] = useState<number | undefined>(settings.proxyPort);
    const [proxyUsername, setProxyUsername] = useState(settings.proxyUsername ?? "");
    const [proxyPassword, setProxyPassword] = useState(settings.proxyPassword ?? "");
    const [sshHost, setSshHost] = useState(settings.sshHost ?? "");
    const [sshPort, setSshPort] = useState<number | undefined>(settings.sshPort ?? 22);
    const [sshUsername, setSshUsername] = useState(settings.sshUsername ?? "");
    const [sshAuthType, setSshAuthType] = useState(settings.sshAuthType ?? "password");
    const [sshPassword, setSshPassword] = useState(settings.sshPassword ?? "");
    const [sshKeyPath, setSshKeyPath] = useState(settings.sshKeyPath ?? "");
    const [sshAutoConnect, setSshAutoConnect] = useState(settings.sshAutoConnect ?? false);
    const [routingEnabled, setRoutingEnabled] = useState(settings.routingEnabled ?? false);
    const [routingStrategy, setRoutingStrategy] = useState<"rules" | "llm">(
        (settings.routingStrategy as "rules" | "llm") ?? "rules"
    );
    const [budgetPlanEnabled, setBudgetPlanEnabled] = useState(settings.budgetPlanEnabled ?? false);
    const [budgetPlanLimit, setBudgetPlanLimit] = useState(settings.budgetPlanLimit ?? 5);
    const [budgetGlobalEnabled, setBudgetGlobalEnabled] = useState(settings.budgetGlobalEnabled ?? false);
    const [budgetGlobalLimit, setBudgetGlobalLimit] = useState(settings.budgetGlobalLimit ?? 10);
    const [budgetGlobalPeriod, setBudgetGlobalPeriod] = useState<"daily" | "monthly">(
        (settings.budgetGlobalPeriod as "daily" | "monthly") ?? "daily"
    );
    const [telegramBotToken, setTelegramBotToken] = useState(settings.telegramBotToken ?? "");
    const [telegramEnabled, setTelegramEnabled] = useState(settings.telegramEnabled ?? false);
    const [telegramAutoStart, setTelegramAutoStart] = useState(settings.telegramAutoStart ?? false);
    const [telegramModel, setTelegramModel] = useState(settings.telegramModel ?? "");
    const [embeddingOpenaiKey, setEmbeddingOpenaiKey] = useState(settings.embeddingOpenaiKey ?? "");
    const [embeddingGeminiKey, setEmbeddingGeminiKey] = useState(settings.embeddingGeminiKey ?? "");
    const [rerankerCohereKey, setRerankerCohereKey] = useState(settings.rerankerCohereKey ?? "");
    const [rerankerJinaKey, setRerankerJinaKey] = useState(settings.rerankerJinaKey ?? "");

    const initialSnapshotRef = useRef<FormSnapshot | null>(null);

    function getFormSnapshot(): FormSnapshot {
        return {
            api_key: apiKey,
            management_key: managementKey,
            temperature,
            max_tokens: maxTokens,
            font_size: fontSize,
            ollamaUrl,
            openrouterEnabledModels: openrouterEnabledModels.slice(0, 5),
            ollamaEnabledModels: ollamaEnabledModels.slice(0, 5),
            customProviderEnabledModels: normalizedCustomProviders(customProviderEnabledModels),
            topP: topPEnabled ? topP : null,
            topK: topKEnabled ? topK : null,
            frequencyPenalty: frequencyPenaltyEnabled ? frequencyPenalty : null,
            presencePenalty: presencePenaltyEnabled ? presencePenalty : null,
            language: language ?? "",
            sendByEnter,
            messageDensity,
            chatWidth,
            showStatusBar,
            statusBarMetrics: statusBarMetrics.slice(),
            webSearchProvider: webSearchProvider ?? "",
            tavilyApiKey: tavilyApiKey ?? "",
            braveApiKey: braveApiKey ?? "",
            terminalFontSize,
            terminalShell,
            proxyEnabled,
            proxyType: proxyType ?? "http",
            proxyHost: proxyHost ?? "",
            proxyPort,
            proxyUsername: proxyUsername ?? "",
            proxyPassword: proxyPassword ?? "",
            sshHost: sshHost ?? "",
            sshPort,
            sshUsername: sshUsername ?? "",
            sshAuthType: sshAuthType ?? "password",
            sshPassword: sshPassword ?? "",
            sshKeyPath: sshKeyPath ?? "",
            sshAutoConnect,
            telegramBotToken,
            telegramEnabled,
            telegramAutoStart,
            telegramModel,
            embeddingOpenaiKey: embeddingOpenaiKey ?? "",
            embeddingGeminiKey: embeddingGeminiKey ?? "",
            rerankerCohereKey: rerankerCohereKey ?? "",
            rerankerJinaKey: rerankerJinaKey ?? "",
        };
    }

    function formSnapshotFromSettings(s: AppSettings): FormSnapshot {
        return {
            api_key: s.api_key,
            management_key: s.management_key,
            temperature: s.temperature,
            max_tokens: s.max_tokens,
            font_size: s.font_size,
            ollamaUrl: s.ollamaUrl,
            openrouterEnabledModels: (s.openrouterEnabledModels ?? []).slice(0, 5),
            ollamaEnabledModels: (s.ollamaEnabledModels ?? []).slice(0, 5),
            customProviderEnabledModels: normalizedCustomProviders(s.customProviderEnabledModels ?? {}),
            topP: s.topP != null ? s.topP : null,
            topK: s.topK != null ? s.topK : null,
            frequencyPenalty: s.frequencyPenalty != null ? s.frequencyPenalty : null,
            presencePenalty: s.presencePenalty != null ? s.presencePenalty : null,
            language: s.language ?? "",
            sendByEnter: s.sendByEnter ?? true,
            messageDensity: s.messageDensity ?? "standard",
            chatWidth: s.chatWidth ?? "standard",
            showStatusBar: s.showStatusBar ?? true,
            statusBarMetrics: (s.statusBarMetrics ?? ["balance", "context", "tokens", "cost"]).slice(),
            webSearchProvider: s.webSearchProvider ?? "",
            tavilyApiKey: s.tavilyApiKey ?? "",
            braveApiKey: s.braveApiKey ?? "",
            terminalFontSize: s.terminalFontSize ?? 13,
            terminalShell: s.terminalShell ?? "",
            proxyEnabled: s.proxyEnabled ?? false,
            proxyType: s.proxyType ?? "http",
            proxyHost: s.proxyHost ?? "",
            proxyPort: s.proxyPort,
            proxyUsername: s.proxyUsername ?? "",
            proxyPassword: s.proxyPassword ?? "",
            sshHost: s.sshHost ?? "",
            sshPort: s.sshPort ?? 22,
            sshUsername: s.sshUsername ?? "",
            sshAuthType: s.sshAuthType ?? "password",
            sshPassword: s.sshPassword ?? "",
            sshKeyPath: s.sshKeyPath ?? "",
            sshAutoConnect: s.sshAutoConnect ?? false,
            telegramBotToken: s.telegramBotToken ?? "",
            telegramModel: s.telegramModel ?? "",
            telegramEnabled: s.telegramEnabled ?? false,
            telegramAutoStart: s.telegramAutoStart ?? false,
            embeddingOpenaiKey: s.embeddingOpenaiKey ?? "",
            embeddingGeminiKey: s.embeddingGeminiKey ?? "",
            rerankerCohereKey: s.rerankerCohereKey ?? "",
            rerankerJinaKey: s.rerankerJinaKey ?? "",
        };
    }

    const currentSnapshot = getFormSnapshot();
    const routingBudgetDirty =
        routingEnabled !== (settings.routingEnabled ?? false) ||
        routingStrategy !== ((settings.routingStrategy as "rules" | "llm") ?? "rules") ||
        budgetPlanEnabled !== (settings.budgetPlanEnabled ?? false) ||
        budgetPlanLimit !== (settings.budgetPlanLimit ?? 5) ||
        budgetGlobalEnabled !== (settings.budgetGlobalEnabled ?? false) ||
        budgetGlobalLimit !== (settings.budgetGlobalLimit ?? 10) ||
        budgetGlobalPeriod !== ((settings.budgetGlobalPeriod as "daily" | "monthly") ?? "daily");
    const isDirty =
        (initialSnapshotRef.current !== null && !isSameSnapshot(currentSnapshot, initialSnapshotRef.current)) ||
        routingBudgetDirty;

    useEffect(() => {
        setApiKey(settings.api_key);
        setManagementKey(settings.management_key);
        setOllamaUrl(settings.ollamaUrl);
        setTemperature(settings.temperature);
        setMaxTokens(settings.max_tokens);
        setFontSize(settings.font_size);
        setOpenrouterEnabledModels(settings.openrouterEnabledModels ?? []);
        setOllamaEnabledModels(settings.ollamaEnabledModels ?? []);
        setCustomProviderEnabledModels(settings.customProviderEnabledModels ?? {});
        setTopP(settings.topP ?? 0.9);
        setTopPEnabled(settings.topP != null);
        setTopK(settings.topK ?? 40);
        setTopKEnabled(settings.topK != null);
        setFrequencyPenalty(settings.frequencyPenalty ?? 0);
        setFrequencyPenaltyEnabled(settings.frequencyPenalty != null);
        setPresencePenalty(settings.presencePenalty ?? 0);
        setPresencePenaltyEnabled(settings.presencePenalty != null);
        setLanguage(settings.language ?? "");
        setSendByEnter(settings.sendByEnter ?? true);
        setMessageDensity(settings.messageDensity ?? "standard");
        setChatWidth(settings.chatWidth ?? "standard");
        setShowStatusBar(settings.showStatusBar ?? true);
        setStatusBarMetrics(settings.statusBarMetrics ?? ["balance", "context", "tokens", "cost"]);
        setWebSearchProvider(settings.webSearchProvider ?? "");
        setTavilyApiKey(settings.tavilyApiKey ?? "");
        setBraveApiKey(settings.braveApiKey ?? "");
        setTerminalFontSize(settings.terminalFontSize ?? 13);
        setTerminalShell(settings.terminalShell ?? "");
        setProxyEnabled(settings.proxyEnabled ?? false);
        setProxyType(settings.proxyType ?? "http");
        setProxyHost(settings.proxyHost ?? "");
        setProxyPort(settings.proxyPort);
        setProxyUsername(settings.proxyUsername ?? "");
        setProxyPassword(settings.proxyPassword ?? "");
        setSshHost(settings.sshHost ?? "");
        setSshPort(settings.sshPort ?? 22);
        setSshUsername(settings.sshUsername ?? "");
        setSshAuthType(settings.sshAuthType ?? "password");
        setSshPassword(settings.sshPassword ?? "");
        setSshKeyPath(settings.sshKeyPath ?? "");
        setSshAutoConnect(settings.sshAutoConnect ?? false);
        setEmbeddingOpenaiKey(settings.embeddingOpenaiKey ?? "");
        setEmbeddingGeminiKey(settings.embeddingGeminiKey ?? "");
        setRerankerCohereKey(settings.rerankerCohereKey ?? "");
        setRerankerJinaKey(settings.rerankerJinaKey ?? "");
        setRoutingEnabled(settings.routingEnabled ?? false);
        setRoutingStrategy((settings.routingStrategy as "rules" | "llm") ?? "rules");
        setBudgetPlanEnabled(settings.budgetPlanEnabled ?? false);
        setBudgetPlanLimit(settings.budgetPlanLimit ?? 5);
        setBudgetGlobalEnabled(settings.budgetGlobalEnabled ?? false);
        setBudgetGlobalLimit(settings.budgetGlobalLimit ?? 10);
        setBudgetGlobalPeriod((settings.budgetGlobalPeriod as "daily" | "monthly") ?? "daily");
        if (initialSnapshotRef.current === null) {
            initialSnapshotRef.current = formSnapshotFromSettings(settings);
        }
    }, [settings]);

    const [isSaving, setIsSaving] = useState(false);

    const handleSave = async () => {
        if (apiKey && apiKey !== settings.api_key) {
            setIsSaving(true);
            try {
                const valid = await invoke<boolean>("validate_openrouter_key", { apiKey });
                if (!valid) {
                    notify.error(t("notifications.apiKeyInvalid"));
                    setIsSaving(false);
                    return;
                }
            } catch {
                notify.warning(t("notifications.apiKeyCheckFailed"));
            } finally {
                setIsSaving(false);
            }
        }
        await saveSettings({
            api_key: apiKey,
            management_key: managementKey,
            model: settings.model,
            temperature,
            max_tokens: maxTokens,
            font_size: fontSize,
            ollamaUrl,
            openrouterEnabledModels: openrouterEnabledModels.slice(0, 5),
            ollamaEnabledModels: ollamaEnabledModels.slice(0, 5),
            customProviderEnabledModels: Object.fromEntries(
                Object.entries(customProviderEnabledModels).map(([k, v]) => [k, v.slice(0, 5)])
            ),
            topP: topPEnabled ? topP : null,
            topK: topKEnabled ? topK : null,
            frequencyPenalty: frequencyPenaltyEnabled ? frequencyPenalty : null,
            presencePenalty: presencePenaltyEnabled ? presencePenalty : null,
            language: language ?? "",
            sendByEnter,
            messageDensity,
            chatWidth,
            showStatusBar,
            statusBarMetrics: statusBarMetrics.slice(),
            sttProvider: settings.sttProvider,
            sttLanguage: settings.sttLanguage,
            openaiApiKey: settings.openaiApiKey,
            webSearchProvider: webSearchProvider || undefined,
            tavilyApiKey: tavilyApiKey || undefined,
            braveApiKey: braveApiKey || undefined,
            terminalFontSize: terminalFontSize,
            terminalShell: terminalShell || undefined,
            proxyEnabled,
            proxyType: proxyType || "http",
            proxyHost: proxyHost || undefined,
            proxyPort,
            proxyUsername: proxyUsername || undefined,
            proxyPassword: proxyPassword || undefined,
            sshHost: sshHost || undefined,
            sshPort: sshPort || 22,
            sshUsername: sshUsername || undefined,
            sshAuthType: sshAuthType || "password",
            sshPassword: sshPassword || undefined,
            sshKeyPath: sshKeyPath || undefined,
            sshAutoConnect,
            telegramBotToken: telegramBotToken || undefined,
            telegramEnabled,
            telegramAutoStart,
            telegramModel: telegramModel || undefined,
            embeddingOpenaiKey: embeddingOpenaiKey || undefined,
            embeddingGeminiKey: embeddingGeminiKey || undefined,
            rerankerCohereKey: rerankerCohereKey || undefined,
            rerankerJinaKey: rerankerJinaKey || undefined,
            routingEnabled,
            routingStrategy: routingStrategy || "rules",
            budgetPlanEnabled,
            budgetPlanLimit,
            budgetGlobalEnabled,
            budgetGlobalLimit,
            budgetGlobalPeriod,
        });
        initialSnapshotRef.current = getFormSnapshot();
        setView("chat");
    };

    const renderSection = () => {
        switch (activeSection) {
            case "openRouter":
                return (
                    <OpenRouterSection
                        apiKey={apiKey}
                        onApiKeyChange={setApiKey}
                        managementKey={managementKey}
                        onManagementKeyChange={setManagementKey}
                        openrouterEnabledModels={openrouterEnabledModels}
                        onOpenrouterEnabledModelsChange={setOpenrouterEnabledModels}
                    />
                );
            case "ollama":
                return (
                    <OllamaModelsSection
                        ollamaUrl={ollamaUrl}
                        onOllamaUrlChange={setOllamaUrl}
                        ollamaEnabledModels={ollamaEnabledModels}
                        onOllamaEnabledModelsChange={setOllamaEnabledModels}
                    />
                );
            case "customProviders":
                return (
                    <CustomProvidersSection
                        customProviderEnabledModels={customProviderEnabledModels}
                        onCustomProviderEnabledModelsChange={setCustomProviderEnabledModels}
                    />
                );
            case "generation":
                return (
                    <GenerationSection
                        temperature={temperature}
                        onTemperatureChange={setTemperature}
                        maxTokens={maxTokens}
                        onMaxTokensChange={setMaxTokens}
                        topP={topP}
                        topPEnabled={topPEnabled}
                        onTopPChange={setTopP}
                        onTopPEnabledChange={setTopPEnabled}
                        topK={topK}
                        topKEnabled={topKEnabled}
                        onTopKChange={setTopK}
                        onTopKEnabledChange={setTopKEnabled}
                        frequencyPenalty={frequencyPenalty}
                        frequencyPenaltyEnabled={frequencyPenaltyEnabled}
                        onFrequencyPenaltyChange={setFrequencyPenalty}
                        onFrequencyPenaltyEnabledChange={setFrequencyPenaltyEnabled}
                        presencePenalty={presencePenalty}
                        presencePenaltyEnabled={presencePenaltyEnabled}
                        onPresencePenaltyChange={setPresencePenalty}
                        onPresencePenaltyEnabledChange={setPresencePenaltyEnabled}
                    />
                );
            case "interface":
                return (
                    <InterfaceSection
                        fontSize={fontSize}
                        onFontSizeChange={setFontSize}
                        language={language}
                        onLanguageChange={setLanguage}
                        sendByEnter={sendByEnter}
                        onSendByEnterChange={setSendByEnter}
                        messageDensity={messageDensity}
                        onMessageDensityChange={(value) => setMessageDensity(value as "compact" | "standard" | "spacious")}
                        chatWidth={chatWidth}
                        onChatWidthChange={(value) => setChatWidth(value as "standard" | "narrow" | "wide")}
                        showStatusBar={showStatusBar}
                        onShowStatusBarChange={setShowStatusBar}
                        statusBarMetrics={statusBarMetrics}
                        onStatusBarMetricsChange={setStatusBarMetrics}
                    />
                );
            case "modes":
                return <ModesSection />;
            case "routing":
                return (
                    <Stack gap="xl">
                        <RoutingSection
                            routingEnabled={routingEnabled}
                            onRoutingEnabledChange={setRoutingEnabled}
                            routingStrategy={routingStrategy}
                            onRoutingStrategyChange={setRoutingStrategy}
                            onSyncModels={() => useChatStore.getState().loadSettings()}
                            lastSync={settings.modelCatalogLastSync ?? null}
                        />
                        <BudgetSection
                            budgetPlanEnabled={budgetPlanEnabled}
                            onBudgetPlanEnabledChange={setBudgetPlanEnabled}
                            budgetPlanLimit={budgetPlanLimit}
                            onBudgetPlanLimitChange={setBudgetPlanLimit}
                            budgetGlobalEnabled={budgetGlobalEnabled}
                            onBudgetGlobalEnabledChange={setBudgetGlobalEnabled}
                            budgetGlobalLimit={budgetGlobalLimit}
                            onBudgetGlobalLimitChange={setBudgetGlobalLimit}
                            budgetGlobalPeriod={budgetGlobalPeriod}
                            onBudgetGlobalPeriodChange={setBudgetGlobalPeriod}
                        />
                    </Stack>
                );
            case "presets":
                return <PresetsSection />;
            case "templates":
                return <TemplatesSection />;
            case "mcp":
                return <McpSection />;
            case "filesystem":
                return <FilesystemSection />;
            case "webSearch":
                return (
                    <WebSearchSection
                        webSearchProvider={webSearchProvider}
                        onWebSearchProviderChange={setWebSearchProvider}
                        tavilyApiKey={tavilyApiKey}
                        onTavilyApiKeyChange={setTavilyApiKey}
                        braveApiKey={braveApiKey}
                        onBraveApiKeyChange={setBraveApiKey}
                    />
                );
            case "proxy":
                return (
                    <VpnSection
                        proxyEnabled={proxyEnabled}
                        onProxyEnabledChange={setProxyEnabled}
                        proxyType={proxyType}
                        onProxyTypeChange={setProxyType}
                        proxyHost={proxyHost}
                        onProxyHostChange={setProxyHost}
                        proxyPort={proxyPort}
                        onProxyPortChange={setProxyPort}
                        proxyUsername={proxyUsername}
                        onProxyUsernameChange={setProxyUsername}
                        proxyPassword={proxyPassword}
                        onProxyPasswordChange={setProxyPassword}
                        sshHost={sshHost}
                        onSshHostChange={setSshHost}
                        sshPort={sshPort}
                        onSshPortChange={setSshPort}
                        sshUsername={sshUsername}
                        onSshUsernameChange={setSshUsername}
                        sshAuthType={sshAuthType}
                        onSshAuthTypeChange={setSshAuthType}
                        sshPassword={sshPassword}
                        onSshPasswordChange={setSshPassword}
                        sshKeyPath={sshKeyPath}
                        onSshKeyPathChange={setSshKeyPath}
                        sshAutoConnect={sshAutoConnect}
                        onSshAutoConnectChange={setSshAutoConnect}
                    />
                );
            case "audio":
                return <AudioSection />;
            case "terminal":
                return (
                    <TerminalSection
                        terminalFontSize={terminalFontSize}
                        onTerminalFontSizeChange={setTerminalFontSize}
                        terminalShell={terminalShell}
                        onTerminalShellChange={setTerminalShell}
                    />
                );
            case "searchModel":
                return <SearchModelSection />;
            case "telegram":
                return (
                    <TelegramSection
                        telegramBotToken={telegramBotToken}
                        onTelegramBotTokenChange={setTelegramBotToken}
                        telegramEnabled={telegramEnabled}
                        onTelegramEnabledChange={setTelegramEnabled}
                        telegramAutoStart={telegramAutoStart}
                        onTelegramAutoStartChange={setTelegramAutoStart}
                        telegramModel={telegramModel}
                        onTelegramModelChange={setTelegramModel}
                    />
                );
            case "embedding":
                return (
                    <Stack gap="lg">
                        <Title order={4}>{t("settings.embeddingKeys")}</Title>
                        <Text size="sm" c="dimmed">{t("settings.embeddingKeysHint")}</Text>
                        <PasswordInput
                            label={t("settings.embeddingOpenaiKey")}
                            placeholder="sk-..."
                            value={embeddingOpenaiKey}
                            onChange={(e) => setEmbeddingOpenaiKey(e.currentTarget.value)}
                        />
                        <PasswordInput
                            label={t("settings.embeddingGeminiKey")}
                            placeholder="AI..."
                            value={embeddingGeminiKey}
                            onChange={(e) => setEmbeddingGeminiKey(e.currentTarget.value)}
                        />

                        <Title order={4} mt="lg">{t("settings.rerankerKeys")}</Title>
                        <Text size="sm" c="dimmed">{t("settings.rerankerKeysHint")}</Text>
                        <PasswordInput
                            label={t("settings.cohereApiKey")}
                            placeholder="..."
                            value={rerankerCohereKey}
                            onChange={(e) => setRerankerCohereKey(e.currentTarget.value)}
                        />
                        <Text size="xs" c="dimmed">{t("settings.getCohereKey")}</Text>
                        <PasswordInput
                            label={t("settings.jinaRerankerKey")}
                            placeholder="jina_..."
                            value={rerankerJinaKey}
                            onChange={(e) => setRerankerJinaKey(e.currentTarget.value)}
                        />
                        <Text size="xs" c="dimmed">{t("settings.getJinaKey")}</Text>
                    </Stack>
                );
            case "data":
                return <DataSection />;
            case "about":
                return <AboutSection />;
            default:
                return null;
        }
    };

    return (
        <Box
            style={{
                height: "100vh",
                display: "flex",
                flexDirection: "column",
                overflow: "hidden",
            }}
        >
            <Group p="md" gap="sm" style={{ flexShrink: 0, borderBottom: "1px solid var(--mantine-color-default-border)" }}>
                <Button variant="subtle" onClick={() => setView("chat")}>
                    ← {t("common.back")}
                </Button>
                <Title order={2} c="brand">{t("settings.title")}</Title>
            </Group>

            <Box
                style={{
                    flex: 1,
                    display: "flex",
                    minHeight: 0,
                }}
            >
                <Box
                    style={{
                        width: 200,
                        flexShrink: 0,
                        borderRight: "1px solid var(--mantine-color-default-border)",
                        overflowY: "auto",
                        minHeight: 0,
                    }}
                >
                    <Stack gap={0} p="xs">
                        {NAV_ITEMS.map(({ section, label, icon }) => (
                            <NavLink
                                key={section}
                                active={activeSection === section}
                                label={label}
                                leftSection={icon}
                                onClick={() => setActiveSection(section)}
                                color="brand"
                            />
                        ))}
                    </Stack>
                </Box>

                <ScrollArea
                    style={{ flex: 1, minWidth: 0 }}
                    type="scroll"
                >
                    <Stack p="lg" gap="lg">
                        {renderSection()}
                        {isDirty && (
                            <Group justify="flex-end">
                                <Button onClick={handleSave} loading={isSaving}>{t("common.save")}</Button>
                            </Group>
                        )}
                    </Stack>
                </ScrollArea>
            </Box>
        </Box>
    );
}

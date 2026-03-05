import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppSettings } from "../types";
import {
    Box,
    Button,
    Group,
    NavLink,
    ScrollArea,
    Stack,
    Title,
} from "@mantine/core";
import {
    IconBox,
    IconCpu,
    IconDatabase,
    IconFileText,
    IconInfoCircle,
    IconKey,
    IconLayoutGrid,
    IconMicrophone,
    IconPalette,
    IconPlug,
    IconSettings,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import {
    AboutSection,
    AudioSection,
    CustomProvidersSection,
    DataSection,
    GenerationSection,
    InterfaceSection,
    McpSection,
    OllamaModelsSection,
    OpenRouterSection,
    PresetsSection,
    TemplatesSection,
} from "./settings";

export type SettingsSection =
    | "openRouter"
    | "ollama"
    | "customProviders"
    | "generation"
    | "interface"
    | "presets"
    | "templates"
    | "mcp"
    | "audio"
    | "data"
    | "about";

const getNavItems = (t: (key: string) => string): { section: SettingsSection; label: string; icon: React.ReactNode }[] => [
    { section: "openRouter", label: t("settings.nav.openRouter"), icon: <IconKey size={18} stroke={1.5} /> },
    { section: "ollama", label: t("settings.nav.ollama"), icon: <IconCpu size={18} stroke={1.5} /> },
    { section: "customProviders", label: t("settings.nav.customProviders"), icon: <IconBox size={18} stroke={1.5} /> },
    { section: "generation", label: t("settings.nav.generation"), icon: <IconSettings size={18} stroke={1.5} /> },
    { section: "interface", label: t("settings.nav.interface"), icon: <IconPalette size={18} stroke={1.5} /> },
    { section: "presets", label: t("settings.nav.presets"), icon: <IconFileText size={18} stroke={1.5} /> },
    { section: "templates", label: t("settings.nav.templates"), icon: <IconLayoutGrid size={18} stroke={1.5} /> },
    { section: "mcp", label: t("settings.nav.mcp"), icon: <IconPlug size={18} stroke={1.5} /> },
    { section: "audio", label: t("settings.nav.audio"), icon: <IconMicrophone size={18} stroke={1.5} /> },
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
        a.presencePenalty !== b.presencePenalty
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
        };
    }

    const currentSnapshot = getFormSnapshot();
    const isDirty =
        initialSnapshotRef.current !== null &&
        !isSameSnapshot(currentSnapshot, initialSnapshotRef.current);

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
        if (initialSnapshotRef.current === null) {
            initialSnapshotRef.current = formSnapshotFromSettings(settings);
        }
    }, [settings]);

    const handleSave = async () => {
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
            case "presets":
                return <PresetsSection />;
            case "templates":
                return <TemplatesSection />;
            case "mcp":
                return <McpSection />;
            case "audio":
                return <AudioSection />;
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
                <Title order={2}>{t("settings.title")}</Title>
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
                                <Button onClick={handleSave}>{t("common.save")}</Button>
                            </Group>
                        )}
                    </Stack>
                </ScrollArea>
            </Box>
        </Box>
    );
}

import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppSettings } from "../types";
import { OpenRouterSettings, GenerationSettings, InterfaceSettings, WebSearchSettings, BudgetSettings, TerminalSettings, OllamaSettings } from "@uni-fw/ui";
import { SshTunnelSettings } from "@uni-fw/ssh-ui";
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
import {
    AboutSection,
    AudioSection,
    CustomProvidersSection,
    DataSection,
    FilesystemSection,
    McpSection,

    PresetsSection,
    RoutingSection,
    TemplatesSection,
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

/** Snapshot of form fields saved via the main Save button (non-migrated fields only). */
interface FormSnapshot {
    customProviderEnabledModels: Record<string, string[]>;
    routingEnabled: boolean;
    routingStrategy: string;
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
        a.routingEnabled !== b.routingEnabled ||
        a.routingStrategy !== b.routingStrategy ||
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
    const aKeys = Object.keys(a.customProviderEnabledModels).sort();
    const bKeys = Object.keys(b.customProviderEnabledModels).sort();
    if (aKeys.length !== bKeys.length || aKeys.some((k, i) => k !== bKeys[i])) return false;
    for (const k of aKeys) {
        const av = a.customProviderEnabledModels[k] ?? [];
        const bv = b.customProviderEnabledModels[k] ?? [];
        if (av.length !== bv.length || av.some((v, i) => v !== bv[i])) return false;
    }
    return true;
}

export function SettingsPage() {
    const { t } = useTranslation();
    const { settings, saveSettings, setView } = useChatStore();
    const NAV_ITEMS = getNavItems(t);

    const [activeSection, setActiveSection] = useState<SettingsSection>("openRouter");

    // Non-migrated local state (these fields still use Save button)
    const [customProviderEnabledModels, setCustomProviderEnabledModels] = useState<
        Record<string, string[]>
    >(settings.customProviderEnabledModels ?? {});
    const [routingEnabled, setRoutingEnabled] = useState(settings.routingEnabled ?? false);
    const [routingStrategy, setRoutingStrategy] = useState<"rules" | "llm">(
        (settings.routingStrategy as "rules" | "llm") ?? "rules"
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
            customProviderEnabledModels: normalizedCustomProviders(customProviderEnabledModels),
            routingEnabled,
            routingStrategy: routingStrategy ?? "rules",
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
            customProviderEnabledModels: normalizedCustomProviders(s.customProviderEnabledModels ?? {}),
            routingEnabled: s.routingEnabled ?? false,
            routingStrategy: (s.routingStrategy as "rules" | "llm") ?? "rules",
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
    const isDirty =
        initialSnapshotRef.current !== null && !isSameSnapshot(currentSnapshot, initialSnapshotRef.current);

    useEffect(() => {
        setCustomProviderEnabledModels(settings.customProviderEnabledModels ?? {});
        setEmbeddingOpenaiKey(settings.embeddingOpenaiKey ?? "");
        setEmbeddingGeminiKey(settings.embeddingGeminiKey ?? "");
        setRerankerCohereKey(settings.rerankerCohereKey ?? "");
        setRerankerJinaKey(settings.rerankerJinaKey ?? "");
        setRoutingEnabled(settings.routingEnabled ?? false);
        setRoutingStrategy((settings.routingStrategy as "rules" | "llm") ?? "rules");
        if (initialSnapshotRef.current === null) {
            initialSnapshotRef.current = formSnapshotFromSettings(settings);
        }
    }, [settings]);

    const [isSaving, setIsSaving] = useState(false);

    const handleSave = async () => {
        setIsSaving(true);
        try {
            await saveSettings({
                ...settings,
                customProviderEnabledModels: Object.fromEntries(
                    Object.entries(customProviderEnabledModels).map(([k, v]) => [k, v.slice(0, 5)])
                ),
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
            });
            initialSnapshotRef.current = getFormSnapshot();
            setView("chat");
        } finally {
            setIsSaving(false);
        }
    };

    const renderSection = () => {
        switch (activeSection) {
            case "openRouter":
                return <OpenRouterSettings />;
            case "ollama":
                return <OllamaSettings />;
            case "customProviders":
                return (
                    <CustomProvidersSection
                        customProviderEnabledModels={customProviderEnabledModels}
                        onCustomProviderEnabledModelsChange={setCustomProviderEnabledModels}
                    />
                );
            case "generation":
                return <GenerationSettings />;
            case "interface":
                return <InterfaceSettings />;
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
                        <BudgetSettings />
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
                return <WebSearchSettings />;
            case "proxy":
                return <SshTunnelSettings />;
            case "audio":
                return <AudioSection />;
            case "terminal":
                return <TerminalSettings />;
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

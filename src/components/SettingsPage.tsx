import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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
    IconPalette,
    IconSettings,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import {
    AboutSection,
    CustomProvidersSection,
    DataSection,
    GenerationSection,
    InterfaceSection,
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
    { section: "data", label: t("settings.nav.data"), icon: <IconDatabase size={18} stroke={1.5} /> },
    { section: "about", label: t("settings.nav.about"), icon: <IconInfoCircle size={18} stroke={1.5} /> },
];

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
        });
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
                    />
                );
            case "presets":
                return <PresetsSection />;
            case "templates":
                return <TemplatesSection />;
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
                        <Group justify="flex-end">
                            <Button onClick={handleSave}>{t("common.save")}</Button>
                        </Group>
                    </Stack>
                </ScrollArea>
            </Box>
        </Box>
    );
}

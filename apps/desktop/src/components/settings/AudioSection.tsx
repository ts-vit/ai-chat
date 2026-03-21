import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Alert,
    Badge,
    Button,
    Group,
    Loader,
    Paper,
    PasswordInput,
    Progress,
    SegmentedControl,
    Select,
    Stack,
    Text,
    Title,
    Tooltip,
} from "@mantine/core";
import { IconCheck, IconDownload, IconInfoCircle } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { ConfirmModal } from "../ConfirmModal";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useChatStore } from "../../store/chatStore";
import { notify } from "../../utils/notify";

interface VoskModelInfo {
    language: string;
    path: string;
    sizeMb: number;
}

interface VoskStatus {
    libraryInstalled: boolean;
    models: VoskModelInfo[];
}

interface DownloadProgress {
    downloaded: number;
    total: number;
    percent: number;
}

const LANGUAGE_OPTIONS = [
    { value: "ru", label: "Русский" },
    { value: "en", label: "English (Английский)" },
    { value: "de", label: "Deutsch (Немецкий)" },
    { value: "fr", label: "Français (Французский)" },
    { value: "es", label: "Español (Испанский)" },
    { value: "zh", label: "中文 (Китайский)" },
    { value: "ja", label: "日本語 (Японский)" },
    { value: "ko", label: "한국어 (Корейский)" },
];

const VOSK_MODEL_SIZE: Record<string, string> = {
    ru: "1.8 GB",
    en: "~50 MB",
    de: "~50 MB",
    fr: "~50 MB",
    es: "~1.5 GB",
    zh: "~50 MB",
    ja: "~50 MB",
    ko: "~50 MB",
};

export function AudioSection() {
    const { t } = useTranslation();
    const { settings, saveSettings } = useChatStore();

    const [sttProvider, setSttProvider] = useState(settings.sttProvider ?? "disabled");
    const [sttLanguage, setSttLanguage] = useState(settings.sttLanguage ?? "en");
    const [openaiApiKey, setOpenaiApiKey] = useState(settings.openaiApiKey ?? "");
    const [groqSttApiKey, setGroqSttApiKey] = useState(settings.groqSttApiKey ?? "");

    const [voskStatus, setVoskStatus] = useState<VoskStatus | null>(null);
    const [installingLib, setInstallingLib] = useState(false);
    const [libProgress, setLibProgress] = useState(0);
    const [downloadingModel, setDownloadingModel] = useState<string | null>(null);
    const [modelProgress, setModelProgress] = useState(0);
    const [uninstallVoskModalOpen, setUninstallVoskModalOpen] = useState(false);

    const [ttsProvider, setTtsProvider] = useState(settings.ttsProvider ?? "system");
    const [ttsVoice, setTtsVoice] = useState(settings.ttsVoice ?? "");
    const [ttsModel, setTtsModel] = useState(settings.ttsModel ?? "tts-1");
    const [systemVoices, setSystemVoices] = useState<{ id: string; name: string; language: string }[]>([]);

    useEffect(() => {
        setSttProvider(settings.sttProvider ?? "disabled");
        setSttLanguage(settings.sttLanguage ?? "en");
        setOpenaiApiKey(settings.openaiApiKey ?? "");
        setGroqSttApiKey(settings.groqSttApiKey ?? "");
        setTtsProvider(settings.ttsProvider ?? "system");
        setTtsVoice(settings.ttsVoice ?? "");
        setTtsModel(settings.ttsModel ?? "tts-1");
    }, [settings]);

    const loadSystemVoices = useCallback(async () => {
        try {
            const list = await invoke<{ id: string; name: string; language: string }[]>("tts_get_voices", {
                provider: "system",
            });
            setSystemVoices(list ?? []);
        } catch {
            setSystemVoices([]);
        }
    }, []);

    useEffect(() => {
        if (ttsProvider === "system") {
            loadSystemVoices();
        }
    }, [ttsProvider, loadSystemVoices]);

    const loadStatus = useCallback(async () => {
        try {
            const status = await invoke<VoskStatus>("check_vosk_status");
            setVoskStatus(status);
        } catch {
            setVoskStatus({ libraryInstalled: false, models: [] });
        }
    }, []);

    useEffect(() => {
        loadStatus();
    }, [loadStatus]);

    useEffect(() => {
        const unlisteners: UnlistenFn[] = [];

        listen<DownloadProgress>("vosk-download-progress", (e) => {
            setLibProgress(Math.round(e.payload.percent));
        }).then((u) => unlisteners.push(u));

        listen<DownloadProgress>("vosk-model-download-progress", (e) => {
            setModelProgress(Math.round(e.payload.percent));
        }).then((u) => unlisteners.push(u));

        return () => {
            unlisteners.forEach((u) => u());
        };
    }, []);

    const handleProviderChange = async (value: string) => {
        const provider = value === "disabled" ? undefined : value;
        setSttProvider(value);
        try {
            await saveSettings({
                ...settings,
                sttProvider: provider,
                sttLanguage,
                openaiApiKey: openaiApiKey || undefined,
                groqSttApiKey: groqSttApiKey || undefined,
            });
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleLanguageChange = async (value: string | null) => {
        if (!value) return;
        setSttLanguage(value);
        try {
            await saveSettings({
                ...settings,
                sttProvider: sttProvider === "disabled" ? undefined : sttProvider,
                sttLanguage: value,
                openaiApiKey: openaiApiKey || undefined,
                groqSttApiKey: groqSttApiKey || undefined,
            });
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleApiKeyChange = (value: string) => {
        setOpenaiApiKey(value);
    };

    const handleApiKeyBlur = async () => {
        try {
            await saveSettings({
                ...settings,
                sttProvider: sttProvider === "disabled" ? undefined : sttProvider,
                sttLanguage,
                openaiApiKey: openaiApiKey || undefined,
                groqSttApiKey: groqSttApiKey || undefined,
            });
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleGroqApiKeyBlur = async () => {
        try {
            await saveSettings({
                ...settings,
                sttProvider: sttProvider === "disabled" ? undefined : sttProvider,
                sttLanguage,
                openaiApiKey: openaiApiKey || undefined,
                groqSttApiKey: groqSttApiKey || undefined,
            });
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleInstallVosk = async () => {
        setInstallingLib(true);
        setLibProgress(0);
        try {
            await invoke("download_vosk_library");
            notify.success(t("settings.audio.voskInstallSuccess"));
            await loadStatus();
        } catch (e) {
            notify.error(`${t("settings.audio.voskInstallError")}: ${e}`);
        } finally {
            setInstallingLib(false);
            setLibProgress(0);
        }
    };

    const handleUninstallVosk = async () => {
        try {
            await invoke("uninstall_vosk");
            notify.success(t("settings.audio.uninstallVoskSuccess"));
            await loadStatus();
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleDownloadModel = async (lang: string) => {
        setDownloadingModel(lang);
        setModelProgress(0);
        try {
            await invoke("download_vosk_model", { language: lang });
            notify.success(t("settings.audio.modelDownloadSuccess"));
            await loadStatus();
        } catch (e) {
            notify.error(`${t("settings.audio.modelDownloadError")}: ${e}`);
        } finally {
            setDownloadingModel(null);
            setModelProgress(0);
        }
    };

    const isModelDownloaded = (lang: string) =>
        voskStatus?.models.some((m) => m.language === lang) ?? false;

    const handleTtsProviderChange = async (value: string | null) => {
        if (!value) return;
        setTtsProvider(value as "system" | "openai");
        try {
            await saveSettings({
                ...settings,
                ttsProvider: value as "system" | "openai",
                ttsVoice: ttsVoice || undefined,
                ttsModel: ttsModel || undefined,
            });
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleTtsVoiceChange = async (value: string | null) => {
        const v = value ?? "";
        setTtsVoice(v);
        try {
            await saveSettings({
                ...settings,
                ttsProvider: ttsProvider as "system" | "openai",
                ttsVoice: v || undefined,
                ttsModel: ttsModel || undefined,
            });
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleTtsModelChange = async (value: string | null) => {
        const v = value ?? "tts-1";
        setTtsModel(v);
        try {
            await saveSettings({
                ...settings,
                ttsProvider: ttsProvider as "system" | "openai",
                ttsVoice: ttsVoice || undefined,
                ttsModel: v || undefined,
            });
        } catch (e) {
            notify.error(String(e));
        }
    };

    const openaiVoices = [
        { value: "alloy", label: "alloy" },
        { value: "echo", label: "echo" },
        { value: "fable", label: "fable" },
        { value: "onyx", label: "onyx" },
        { value: "nova", label: "nova" },
        { value: "shimmer", label: "shimmer" },
    ];
    const ttsModelOptions = [
        { value: "tts-1", label: t("settings.audio.ttsModelTts1") },
        { value: "tts-1-hd", label: t("settings.audio.ttsModelTts1Hd") },
    ];

    return (
        <Stack gap="lg">
            <Title order={4}>{t("settings.audio.title")}</Title>

            <Stack gap="sm">
                <Text size="sm" fw={500}>
                    {t("settings.audio.sttProvider")}
                </Text>
                <SegmentedControl
                    value={sttProvider}
                    onChange={handleProviderChange}
                    data={[
                        { value: "disabled", label: t("settings.audio.disabled") },
                        { value: "vosk", label: t("settings.audio.vosk") },
                        { value: "whisper", label: t("settings.audio.whisper") },
                        { value: "groq", label: t("settings.audio.groqWhisper") },
                    ]}
                />
            </Stack>

            <Select
                label={t("settings.audio.language")}
                value={sttLanguage}
                onChange={handleLanguageChange}
                data={LANGUAGE_OPTIONS}
                allowDeselect={false}
            />

            {sttProvider === "vosk" && (
                <Paper p="md" withBorder>
                    <Stack gap="md">
                        <Text size="sm" c="dimmed">
                            {t("settings.audio.voskDescription")}
                        </Text>

                        {/* Library status */}
                        <Group justify="space-between" align="center">
                            <Text size="sm" fw={500}>
                                {t("settings.audio.voskLibrary")}
                            </Text>
                            {voskStatus?.libraryInstalled ? (
                                <Group gap="xs" align="center">
                                    <Badge color="green" variant="light" size="lg" leftSection={<IconCheck size={14} />}>
                                        {t("settings.audio.voskLibraryInstalled")}
                                    </Badge>
                                    <Button
                                        size="xs"
                                        variant="subtle"
                                        color="red"
                                        onClick={() => setUninstallVoskModalOpen(true)}
                                    >
                                        {t("settings.audio.uninstallVosk")}
                                    </Button>
                                </Group>
                            ) : (
                                <Button
                                    size="xs"
                                    variant="filled"
                                    loading={installingLib}
                                    leftSection={!installingLib ? <IconDownload size={14} /> : undefined}
                                    onClick={handleInstallVosk}
                                >
                                    {installingLib
                                        ? t("settings.audio.installingVosk")
                                        : t("settings.audio.installVosk")}
                                </Button>
                            )}
                        </Group>

                        {installingLib && libProgress > 0 && (
                            <Progress value={libProgress} size="sm" animated />
                        )}

                        {/* Models */}
                        {voskStatus?.libraryInstalled && (
                            <>
                                <Text size="sm" fw={500} mt="xs">
                                    {t("settings.audio.voskModelsTitle")}
                                </Text>
                                {voskStatus?.libraryInstalled && !isModelDownloaded("ru") && (
                                    <Alert
                                        variant="light"
                                        color="yellow"
                                        icon={<IconInfoCircle size={18} stroke={1.5} />}
                                        title={t("settings.audio.modelSizeWarning")}
                                    />
                                )}
                                <Stack gap="xs">
                                    {LANGUAGE_OPTIONS.map((lang) => {
                                        const downloaded = isModelDownloaded(lang.value);
                                        const isDownloading = downloadingModel === lang.value;
                                        return (
                                            <Group
                                                key={lang.value}
                                                justify="space-between"
                                                align="center"
                                                py={4}
                                                style={{
                                                    borderBottom: "1px solid var(--mantine-color-default-border)",
                                                }}
                                            >
                                                <Group gap="xs" wrap="nowrap">
                                                    <Text size="sm">{lang.label}</Text>
                                                    <Text size="xs" c="dimmed">
                                                        {VOSK_MODEL_SIZE[lang.value] ?? "—"}
                                                    </Text>
                                                </Group>
                                                {downloaded ? (
                                                    <Badge color="green" variant="light" size="sm">
                                                        {t("settings.audio.modelDownloaded")}
                                                    </Badge>
                                                ) : isDownloading ? (
                                                    <Group gap="xs" align="center">
                                                        <Text size="xs" c="dimmed">
                                                            {modelProgress}%
                                                        </Text>
                                                        <Loader size="xs" />
                                                    </Group>
                                                ) : (
                                                    <Tooltip label={t("settings.audio.downloadModel")}>
                                                        <ActionIcon
                                                            size="sm"
                                                            variant="subtle"
                                                            onClick={() => handleDownloadModel(lang.value)}
                                                        >
                                                            <IconDownload size={16} stroke={1.5} />
                                                        </ActionIcon>
                                                    </Tooltip>
                                                )}
                                            </Group>
                                        );
                                    })}
                                </Stack>
                                {isDownloading(downloadingModel) && modelProgress > 0 && (
                                    <Progress value={modelProgress} size="sm" animated />
                                )}
                            </>
                        )}
                    </Stack>
                </Paper>
            )}

            <ConfirmModal
                opened={uninstallVoskModalOpen}
                onClose={() => setUninstallVoskModalOpen(false)}
                onConfirm={handleUninstallVosk}
                message={t("settings.audio.uninstallVoskConfirm")}
                confirmLabel={t("settings.audio.uninstallVosk")}
            />

            {sttProvider === "whisper" && (
                <Paper p="md" withBorder>
                    <Stack gap="sm">
                        <Text size="sm" c="dimmed">
                            {t("settings.audio.whisperDescription")}
                        </Text>
                        <PasswordInput
                            label={t("settings.audio.openaiApiKey")}
                            value={openaiApiKey}
                            onChange={(e) => handleApiKeyChange(e.currentTarget.value)}
                            onBlur={handleApiKeyBlur}
                            placeholder="sk-..."
                        />
                    </Stack>
                </Paper>
            )}

            {sttProvider === "groq" && (
                <Paper p="md" withBorder>
                    <Stack gap="sm">
                        <Text size="sm" fw={500}>
                            {t("settings.audio.groqTitle")}
                        </Text>
                        <Text size="sm" c="dimmed">
                            {t("settings.audio.groqDescription")}
                        </Text>
                        <PasswordInput
                            label={t("settings.audio.groqApiKey")}
                            value={groqSttApiKey}
                            onChange={(e) => setGroqSttApiKey(e.currentTarget.value)}
                            onBlur={handleGroqApiKeyBlur}
                            placeholder="gsk_..."
                        />
                        <Text
                            component="a"
                            href="https://console.groq.com/keys"
                            target="_blank"
                            rel="noopener noreferrer"
                            size="xs"
                            c="blue"
                            style={{ textDecoration: "underline" }}
                        >
                            {t("settings.audio.groqGetKey")}
                        </Text>
                    </Stack>
                </Paper>
            )}

            <Stack gap="sm">
                <Text size="sm" fw={500}>
                    {t("settings.audio.ttsTitle")}
                </Text>
                <Select
                    label={t("settings.audio.ttsProvider")}
                    value={ttsProvider}
                    onChange={(val) => handleTtsProviderChange(val)}
                    data={[
                        { value: "system", label: t("settings.audio.ttsSystem") },
                        { value: "openai", label: t("settings.audio.ttsOpenai") },
                    ]}
                    allowDeselect={false}
                />
                {ttsProvider === "system" && (
                    <Select
                        label={t("settings.audio.ttsVoice")}
                        value={ttsVoice || (systemVoices[0]?.id ?? null)}
                        onChange={handleTtsVoiceChange}
                        data={systemVoices.map((v) => ({ value: v.id, label: `${v.name} (${v.language})` }))}
                        allowDeselect={false}
                        placeholder={systemVoices.length === 0 ? t("common.loading") : undefined}
                    />
                )}
                {ttsProvider === "openai" && (
                    <>
                        <Text size="xs" c="dimmed">
                            {t("settings.audio.ttsOpenaiKeyHint")}
                        </Text>
                        <Select
                            label={t("settings.audio.ttsVoice")}
                            value={ttsVoice || "alloy"}
                            onChange={handleTtsVoiceChange}
                            data={openaiVoices}
                            allowDeselect={false}
                        />
                        <Select
                            label={t("settings.audio.ttsModel")}
                            value={ttsModel}
                            onChange={handleTtsModelChange}
                            data={ttsModelOptions}
                            allowDeselect={false}
                        />
                    </>
                )}
            </Stack>
        </Stack>
    );
}

function isDownloading(lang: string | null): lang is string {
    return lang !== null;
}

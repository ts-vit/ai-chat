import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Button,
    Checkbox,
    Group,
    Loader,
    Progress,
    Select,
    Stack,
    Text,
    TextInput,
    ThemeIcon,
    Tooltip,
} from "@mantine/core";
import { IconCircleCheck, IconCircleX, IconTrash } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { useChatStore } from "../store/chatStore";
import { notify } from "../utils/notify";
import { ConfirmModal } from "./ConfirmModal";

const PULL_MODEL_KEYS = [
    { value: "llama3.2", labelKey: "ollama.pullLlama" as const },
    { value: "qwen2.5:0.5b", labelKey: "ollama.pullQwen" as const },
    { value: "mistral", labelKey: "ollama.pullMistral" as const },
    { value: "phi3", labelKey: "ollama.pullPhi" as const },
    { value: "gemma2:2b", labelKey: "ollama.pullGemma" as const },
] as const;

const MAX_ENABLED_MODELS = 5;

interface OllamaSectionProps {
    ollamaUrl: string;
    onOllamaUrlChange: (url: string) => void;
    ollamaEnabledModels: string[];
    onOllamaEnabledModelsChange: (ids: string[]) => void;
}

export function OllamaSection({ ollamaUrl, onOllamaUrlChange, ollamaEnabledModels, onOllamaEnabledModelsChange }: OllamaSectionProps) {
    const { t } = useTranslation();
    const {
        ollamaStatus,
        localOllamaModels,
        checkOllamaStatus,
        loadLocalOllamaModels,
        deleteOllamaModel,
    } = useChatStore();

    const [deletingModelName, setDeletingModelName] = useState<string | null>(null);
    const [selectedModel, setSelectedModel] = useState<string>("llama3.2");
    const [isPulling, setIsPulling] = useState(false);
    const [pullProgress, setPullProgress] = useState(0);
    const [pullStatus, setPullStatus] = useState("");
    const unlistenRef = useRef<Array<() => void>>([]);

    useEffect(() => {
        checkOllamaStatus();
    }, [checkOllamaStatus]);

    useEffect(() => {
        if (ollamaStatus === "available") {
            loadLocalOllamaModels();
        }
    }, [ollamaStatus, loadLocalOllamaModels]);

    useEffect(() => {
        return () => {
            unlistenRef.current.forEach((fn) => fn());
            unlistenRef.current = [];
        };
    }, []);

    const handleInstallOllama = () => {
        open("https://ollama.com");
    };

    const handlePull = async () => {
        setIsPulling(true);
        setPullProgress(0);
        setPullStatus("");

        const cleanup = () => {
            unlistenRef.current.forEach((fn) => fn());
            unlistenRef.current = [];
        };

        try {
            const unprogress = await listen<{
                status?: string;
                completed?: number;
                total?: number;
            }>("ollama-pull-progress", (event) => {
                const { status, completed, total } = event.payload;
                if (status) setPullStatus(status);
                if (typeof total === "number" && total > 0 && typeof completed === "number") {
                    setPullProgress(Math.round((completed / total) * 100));
                }
            });
            const undone = await listen<{ model: string }>("ollama-pull-done", () => {
                setIsPulling(false);
                loadLocalOllamaModels();
                notify.success(t("notifications.modelDownloaded"));
                cleanup();
            });
            const unerror = await listen<{ message: string }>("ollama-pull-error", (event) => {
                setIsPulling(false);
                notify.error((event.payload.message ?? "") ? `${t("notifications.error")}: ${event.payload.message}` : t("notifications.error"));
                cleanup();
            });

            unlistenRef.current = [unprogress, undone, unerror];
            await invoke("pull_ollama_model", { modelName: selectedModel });
        } catch (e) {
            setIsPulling(false);
            notify.error(String(e));
            cleanup();
        }
    };

    const handleCancelPull = () => {
        setIsPulling(false);
        unlistenRef.current.forEach((fn) => fn());
        unlistenRef.current = [];
    };

    const handleConfirmDelete = () => {
        if (deletingModelName) {
            deleteOllamaModel(deletingModelName);
            setDeletingModelName(null);
        }
    };

    const isModelInstalled = localOllamaModels.some((m) => m.name === selectedModel);

    return (
        <Stack gap="xs">
            <Text size="sm" fw={500}>
                {t("ollama.title")}
            </Text>

            {/* Статус */}
            <Group gap="xs">
                {ollamaStatus === "unknown" && (
                    <>
                        <Loader size="xs" />
                        <Text size="sm">{t("ollama.checking")}</Text>
                    </>
                )}
                {ollamaStatus === "available" && (
                    <>
                        <ThemeIcon size="sm" color="green">
                            <IconCircleCheck size={16} stroke={1.5} />
                        </ThemeIcon>
                        <Text size="sm">{t("ollama.running")}</Text>
                    </>
                )}
                {ollamaStatus === "unavailable" && (
                    <>
                        <IconCircleX size={18} stroke={1.5} color="var(--mantine-color-red-6)" />
                        <Text size="sm">{t("ollama.notFound")}</Text>
                        <Button
                            variant="light"
                            size="xs"
                            onClick={handleInstallOllama}
                        >
                            {t("ollama.installOllama")}
                        </Button>
                    </>
                )}
            </Group>

            {ollamaStatus === "available" && (
                <>
                    {/* Установленные модели */}
                    <Text size="sm" fw={500} mt="xs">
                        {t("ollama.installedModels")}
                    </Text>
                    {localOllamaModels.length === 0 ? (
                        <Text size="sm" c="dimmed">
                            {t("ollama.noInstalledModels")}
                        </Text>
                    ) : (
                        <Stack gap="xs">
                            <Text size="xs" c="dimmed">
                                {t("ollama.selectedCount", { count: ollamaEnabledModels.length, total: MAX_ENABLED_MODELS })}
                            </Text>
                            {localOllamaModels.map((m) => {
                                const checked = ollamaEnabledModels.includes(m.name);
                                const disabled = !checked && ollamaEnabledModels.length >= MAX_ENABLED_MODELS;
                                return (
                                    <Group key={m.name} justify="space-between">
                                        <Group gap="xs">
                                            <Checkbox
                                                checked={checked}
                                                disabled={disabled}
                                                onChange={() => {
                                                    if (checked) {
                                                        onOllamaEnabledModelsChange(ollamaEnabledModels.filter((id) => id !== m.name));
                                                    } else if (ollamaEnabledModels.length < MAX_ENABLED_MODELS) {
                                                        onOllamaEnabledModelsChange([...ollamaEnabledModels, m.name]);
                                                    }
                                                }}
                                                label={t("ollama.showWhenSelectingChat")}
                                                size="xs"
                                            />
                                            <Text size="sm" fw={500}>
                                                {m.name}
                                            </Text>
                                            <Text size="sm" c="dimmed">
                                                {m.size}
                                            </Text>
                                        </Group>
                                        <Tooltip label={t("ollama.deleteModel")}>
                                            <ActionIcon
                                                size="xs"
                                                variant="subtle"
                                                onClick={() => setDeletingModelName(m.name)}
                                                aria-label={t("ollama.deleteModel")}
                                            >
                                                <IconTrash size={16} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                );
                            })}
                        </Stack>
                    )}

                    {/* Скачать модель */}
                    <Text size="sm" fw={500} mt="xs">
                        {t("ollama.downloadModel")}
                    </Text>
                    <Group align="flex-end" gap="sm">
                        <Select
                            label={null}
                            data={PULL_MODEL_KEYS.map((p) => ({ value: p.value, label: t(p.labelKey) }))}
                            value={selectedModel}
                            onChange={(v) => v && setSelectedModel(v)}
                            allowDeselect={false}
                            style={{ minWidth: 280 }}
                        />
                        <Button
                            variant="filled"
                            onClick={handlePull}
                            disabled={isModelInstalled || isPulling}
                        >
                            {t("ollama.download")}
                        </Button>
                    </Group>

                    {isPulling && (
                        <Stack gap="xs">
                            <Progress value={pullProgress} size="sm" animated />
                            <Text size="xs" c="dimmed">
                                {pullStatus || t("common.loading")}
                            </Text>
                            <Button variant="subtle" size="xs" onClick={handleCancelPull}>
                                {t("ollama.cancel")}
                            </Button>
                        </Stack>
                    )}

                    {/* URL сервера */}
                    <TextInput
                        label={t("ollama.serverUrl")}
                        placeholder={t("ollama.serverUrlPlaceholder")}
                        value={ollamaUrl}
                        onChange={(e) => onOllamaUrlChange(e.currentTarget.value)}
                        mt="xs"
                    />
                    <Text size="xs" c="dimmed">
                        {t("ollama.serverUrlHint")}
                    </Text>
                </>
            )}

            <ConfirmModal
                opened={deletingModelName !== null}
                onClose={() => setDeletingModelName(null)}
                onConfirm={handleConfirmDelete}
                message={
                    deletingModelName
                        ? t("ollama.deleteModelConfirm", { name: deletingModelName })
                        : ""
                }
            />
        </Stack>
    );
}

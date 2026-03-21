import { useEffect, useState } from "react";
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
    ThemeIcon,
    Tooltip,
} from "@mantine/core";
import { IconCircleCheck, IconCircleX, IconTrash } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-shell";
import { useChatStore } from "../store/chatStore";
import { ConfirmModal } from "./ConfirmModal";

const PULL_MODEL_KEYS = [
    { value: "qwen2.5:0.5b", labelKey: "ollama.pullQwen05" as const },
    { value: "qwen2.5:1.5b", labelKey: "ollama.pullQwen15" as const },
    { value: "gemma2:2b", labelKey: "ollama.pullGemma" as const },
    { value: "qwen2.5:3b", labelKey: "ollama.pullQwen3" as const },
    { value: "qwen2.5-coder:3b", labelKey: "ollama.pullQwenCoder" as const },
    { value: "llama3.2:3b", labelKey: "ollama.pullLlama" as const },
    { value: "phi3:latest", labelKey: "ollama.pullPhi" as const },
] as const;

const MAX_ENABLED_MODELS = 5;

interface OllamaSectionProps {
    ollamaUrl: string;
    onOllamaUrlChange: (url: string) => void;
    ollamaEnabledModels: string[];
    onOllamaEnabledModelsChange: (ids: string[]) => void;
}

export function OllamaSection({ ollamaUrl: _ollamaUrl, onOllamaUrlChange: _onOllamaUrlChange, ollamaEnabledModels, onOllamaEnabledModelsChange }: OllamaSectionProps) {
    const { t } = useTranslation();
    const {
        ollamaStatus,
        localOllamaModels,
        ollamaPullProgress,
        checkOllamaStatus,
        loadLocalOllamaModels,
        deleteOllamaModel,
        pullOllamaModel,
        clearOllamaPullProgress,
    } = useChatStore();

    const [deletingModelName, setDeletingModelName] = useState<string | null>(null);
    const [selectedModel, setSelectedModel] = useState<string>("qwen2.5:3b");

    useEffect(() => {
        checkOllamaStatus();
    }, [checkOllamaStatus]);

    useEffect(() => {
        if (ollamaStatus === "available") {
            loadLocalOllamaModels();
        }
    }, [ollamaStatus, loadLocalOllamaModels]);

    const handleInstallOllama = () => {
        open("https://ollama.com");
    };

    const handlePull = () => {
        pullOllamaModel(selectedModel);
    };

    const isPulling = ollamaPullProgress !== null;

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
                            style={{ minWidth: 350 }}
                            styles={{ dropdown: { minWidth: 350 } }}
                        />
                        <Button
                            variant="filled"
                            onClick={handlePull}
                            disabled={isModelInstalled || isPulling}
                        >
                            {t("ollama.download")}
                        </Button>
                    </Group>

                    {ollamaPullProgress && (
                        <Stack gap="xs">
                            <Progress value={ollamaPullProgress.progress} size="sm" animated />
                            <Text size="xs" c="dimmed">
                                {ollamaPullProgress.status || t("common.loading")}
                            </Text>
                            <Button variant="subtle" size="xs" onClick={clearOllamaPullProgress}>
                                {t("ollama.cancel")}
                            </Button>
                        </Stack>
                    )}

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

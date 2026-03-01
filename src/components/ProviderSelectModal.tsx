import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Accordion,
    Badge,
    Button,
    Group,
    Modal,
    ScrollArea,
    Stack,
    Text,
    TextInput,
    UnstyledButton,
} from "@mantine/core";
import {
    IconCloud,
    IconPlugConnected,
    IconSearch,
    IconServer,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import type { ModelInfo } from "../types";

interface ProviderSelectModalProps {
    opened: boolean;
    onClose: () => void;
    onConfirm: (providerId: string, model: string, isImageModel: boolean) => void;
}

function getOpenRouterDisplayName(modelId: string): string {
    const lastSlash = modelId.lastIndexOf("/");
    return lastSlash >= 0 ? modelId.slice(lastSlash + 1) : modelId;
}

function isFreeModel(modelId: string, modelInfo: ModelInfo | undefined): boolean {
    if (modelId.endsWith(":free")) return true;
    if (!modelInfo?.pricing) return false;
    return (
        modelInfo.pricing.prompt === "0" && modelInfo.pricing.completion === "0"
    );
}

interface ProviderEntry {
    id: string;
    name: string;
    icon: React.ReactNode;
    models: string[];
    isOpenRouter: boolean;
}

export function ProviderSelectModal({
    opened,
    onClose,
    onConfirm,
}: ProviderSelectModalProps) {
    const { t } = useTranslation();
    const {
        settings,
        customProviders,
        ollamaStatus,
        checkOllamaStatus,
        models,
        loadModels,
    } = useChatStore();
    const [selectedProviderId, setSelectedProviderId] = useState<string | null>(
        null
    );
    const [selectedModel, setSelectedModel] = useState<string>("");
    const [searchQuery, setSearchQuery] = useState<string>("");

    const openrouterModels = settings.openrouterEnabledModels ?? [];
    const ollamaModels = settings.ollamaEnabledModels ?? [];
    const showOllama = ollamaStatus === "available";
    const customEnabledModels = settings.customProviderEnabledModels ?? {};

    const totalModelsCount = useMemo(() => {
        const customTotal = customProviders.reduce(
            (acc, p) => acc + (customEnabledModels[p.id]?.length ?? 0),
            0
        );
        return (
            openrouterModels.length +
            (showOllama ? ollamaModels.length : 0) +
            customTotal
        );
    }, [
        openrouterModels.length,
        showOllama,
        ollamaModels.length,
        customProviders,
        customEnabledModels,
    ]);

    const providersWithModels = useMemo((): ProviderEntry[] => {
        const q = searchQuery.trim().toLowerCase();
        const filter = (list: string[]) =>
            q ? list.filter((id) => id.toLowerCase().includes(q)) : list;

        const entries: ProviderEntry[] = [
            {
                id: "openrouter",
                name: "OpenRouter",
                icon: <IconCloud size={18} stroke={1.5} />,
                models: filter(openrouterModels),
                isOpenRouter: true,
            },
        ];
        if (showOllama) {
            entries.push({
                id: "ollama",
                name: "Ollama",
                icon: <IconServer size={18} stroke={1.5} />,
                models: filter(ollamaModels),
                isOpenRouter: false,
            });
        }
        customProviders.forEach((p) => {
            const list = customEnabledModels[p.id] ?? [];
            entries.push({
                id: p.id,
                name: p.name,
                icon: <IconPlugConnected size={18} stroke={1.5} />,
                models: filter(list),
                isOpenRouter: false,
            });
        });
        return entries;
    }, [
        searchQuery,
        openrouterModels,
        showOllama,
        ollamaModels,
        customProviders,
        customEnabledModels,
    ]);

    const defaultValue = useMemo(() => {
        const firstWithModels = providersWithModels.find(
            (p) => p.models.length > 0
        );
        if (firstWithModels) return [firstWithModels.id];
        return providersWithModels[0]
            ? [providersWithModels[0].id]
            : ([] as string[]);
    }, [providersWithModels]);

    function getOriginalModelCount(entry: ProviderEntry): number {
        if (entry.id === "openrouter") return openrouterModels.length;
        if (entry.id === "ollama") return ollamaModels.length;
        const list = customEnabledModels[entry.id] ?? [];
        return list.length;
    }

    useEffect(() => {
        if (!opened) {
            setSelectedProviderId(null);
            setSelectedModel("");
            setSearchQuery("");
        } else {
            checkOllamaStatus();
            loadModels();
        }
    }, [opened, checkOllamaStatus, loadModels]);

    useEffect(() => {
        if (!selectedProviderId || !selectedModel) return;
        const entry = providersWithModels.find((e) => e.id === selectedProviderId);
        const stillVisible =
            entry?.models.some((m) => m === selectedModel) ?? false;
        if (!stillVisible) {
            setSelectedProviderId(null);
            setSelectedModel("");
        }
    }, [
        searchQuery,
        selectedProviderId,
        selectedModel,
        providersWithModels,
        openrouterModels,
        ollamaModels,
        customEnabledModels,
    ]);

    const handleSelect = (providerId: string, model: string) => {
        setSelectedProviderId(providerId);
        setSelectedModel(model);
    };

    const handleConfirm = () => {
        if (selectedProviderId && selectedModel) {
            const modelInfo = models.find((m) => m.id === selectedModel);
            const isImageModel = modelInfo?.supportsImageGeneration ?? false;
            onConfirm(selectedProviderId, selectedModel, isImageModel);
            onClose();
        }
    };

    const canConfirm = !!selectedProviderId && !!selectedModel;

    return (
        <Modal
            title={t("providerModal.title")}
            size="md"
            opened={opened}
            onClose={onClose}
        >
            <Stack gap="md">
                <Text size="sm" c="dimmed">
                    {t("providerModal.hint")}
                </Text>
                {totalModelsCount > 5 && (
                    <TextInput
                        placeholder={t("providerModal.searchPlaceholder")}
                        leftSection={<IconSearch size={16} stroke={1.5} />}
                        value={searchQuery}
                        onChange={(e) =>
                            setSearchQuery(e.currentTarget.value)
                        }
                    />
                )}
                <ScrollArea.Autosize mah={400} type="scroll">
                    <Accordion
                        variant="separated"
                        multiple
                        defaultValue={defaultValue}
                    >
                        {providersWithModels
                            .filter((entry) => {
                                const originalCount =
                                    getOriginalModelCount(entry);
                                return (
                                    originalCount === 0 ||
                                    entry.models.length > 0
                                );
                            })
                            .map((entry) => {
                                const originalCount =
                                    getOriginalModelCount(entry);
                                const hasNoModels = originalCount === 0;

                                return (
                                    <Accordion.Item
                                        key={entry.id}
                                        value={entry.id}
                                    >
                                    <Accordion.Control>
                                        <Group gap="xs" wrap="nowrap">
                                            {entry.icon}
                                            <Text size="sm" fw={600}>
                                                {entry.name}
                                            </Text>
                                            <Badge
                                                size="sm"
                                                variant="default"
                                                circle
                                            >
                                                {entry.models.length}
                                            </Badge>
                                        </Group>
                                    </Accordion.Control>
                                    <Accordion.Panel>
                                        {hasNoModels ? (
                                            <Text size="xs" c="dimmed">
                                                {t(
                                                    "providerModal.selectInSettings"
                                                )}
                                            </Text>
                                        ) : (
                                            <Stack gap={4}>
                                                {entry.models.map((modelId) => {
                                                    const selected =
                                                        selectedProviderId ===
                                                            entry.id &&
                                                        selectedModel ===
                                                            modelId;
                                                    const displayName =
                                                        entry.isOpenRouter
                                                            ? getOpenRouterDisplayName(
                                                                  modelId
                                                              )
                                                            : modelId;
                                                    const modelInfo = entry.isOpenRouter
                                                        ? models.find(
                                                              (m) =>
                                                                  m.id ===
                                                                  modelId
                                                          )
                                                        : undefined;
                                                    const isFree =
                                                        entry.isOpenRouter &&
                                                        isFreeModel(
                                                            modelId,
                                                            modelInfo
                                                        );
                                                    const isImage =
                                                        !!modelInfo?.supportsImageGeneration;

                                                    return (
                                                        <UnstyledButton
                                                            key={modelId}
                                                            onClick={() =>
                                                                handleSelect(
                                                                    entry.id,
                                                                    modelId
                                                                )
                                                            }
                                                            style={{
                                                                padding:
                                                                    "6px 10px",
                                                                borderRadius:
                                                                    "var(--mantine-radius-sm)",
                                                                backgroundColor: selected
                                                                    ? "var(--mantine-color-blue-light)"
                                                                    : undefined,
                                                            }}
                                                        >
                                                            <Group
                                                                gap="xs"
                                                                justify="space-between"
                                                                wrap="nowrap"
                                                            >
                                                                <Text
                                                                    size="sm"
                                                                    lineClamp={1}
                                                                    style={{
                                                                        flex: 1,
                                                                    }}
                                                                >
                                                                    {
                                                                        displayName
                                                                    }
                                                                </Text>
                                                                <Group
                                                                    gap={4}
                                                                    wrap="nowrap"
                                                                >
                                                                    {isFree && (
                                                                        <Badge
                                                                            size="xs"
                                                                            variant="light"
                                                                            color="green"
                                                                        >
                                                                            free
                                                                        </Badge>
                                                                    )}
                                                                    {isImage && (
                                                                        <Badge
                                                                            size="xs"
                                                                            variant="light"
                                                                            color="violet"
                                                                        >
                                                                            🖼️
                                                                        </Badge>
                                                                    )}
                                                                </Group>
                                                            </Group>
                                                        </UnstyledButton>
                                                    );
                                                })}
                                            </Stack>
                                        )}
                                    </Accordion.Panel>
                                </Accordion.Item>
                            );
                        })}
                    </Accordion>
                </ScrollArea.Autosize>
                <Group justify="flex-end" gap="sm">
                    <Button variant="subtle" onClick={onClose}>
                        {t("providerModal.cancel")}
                    </Button>
                    <Button
                        disabled={!canConfirm}
                        onClick={handleConfirm}
                    >
                        {t("providerModal.create")}
                    </Button>
                </Group>
            </Stack>
        </Modal>
    );
}

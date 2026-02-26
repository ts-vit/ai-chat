import { useEffect, useState } from "react";
import { Box, Button, Group, Modal, ScrollArea, Stack, Text } from "@mantine/core";
import { useChatStore } from "../store/chatStore";

interface ProviderSelectModalProps {
    opened: boolean;
    onClose: () => void;
    onConfirm: (providerId: string, model: string) => void;
}

export function ProviderSelectModal({
    opened,
    onClose,
    onConfirm,
}: ProviderSelectModalProps) {
    const { settings, customProviders, ollamaStatus, checkOllamaStatus } = useChatStore();
    const [selectedProviderId, setSelectedProviderId] = useState<string | null>(null);
    const [selectedModel, setSelectedModel] = useState<string>("");

    const openrouterModels = settings.openrouterEnabledModels ?? [];
    const ollamaModels = settings.ollamaEnabledModels ?? [];
    const showOllama = ollamaStatus === "available";

    useEffect(() => {
        if (!opened) {
            setSelectedProviderId(null);
            setSelectedModel("");
        } else {
            checkOllamaStatus();
        }
    }, [opened, checkOllamaStatus]);

    const handleSelect = (providerId: string, model: string) => {
        setSelectedProviderId(providerId);
        setSelectedModel(model);
    };

    const handleConfirm = () => {
        if (selectedProviderId && selectedModel) {
            onConfirm(selectedProviderId, selectedModel);
            onClose();
        }
    };

    const canConfirm = !!selectedProviderId && !!selectedModel;

    return (
        <Modal
            title="Новый чат"
            size="sm"
            opened={opened}
            onClose={onClose}
        >
            <Stack gap="md">
                <Text size="sm" c="dimmed">
                    Выберите провайдер и модель для чата
                </Text>
                <ScrollArea.Autosize mah={320} type="scroll">
                    <Stack gap="lg">
                        {/* OpenRouter */}
                        <Stack gap="xs">
                            <Text size="sm" fw={600}>
                                OpenRouter
                            </Text>
                            {openrouterModels.length === 0 ? (
                                <Text size="xs" c="dimmed">
                                    Выберите модели в настройках
                                </Text>
                            ) : (
                                openrouterModels.map((modelId) => {
                                    const selected = selectedProviderId === "openrouter" && selectedModel === modelId;
                                    return (
                                        <Box
                                            key={modelId}
                                            onClick={() => handleSelect("openrouter", modelId)}
                                            style={{
                                                padding: "8px 12px",
                                                borderRadius: "var(--mantine-radius-sm)",
                                                cursor: "pointer",
                                                backgroundColor: selected ? "var(--mantine-color-blue-light)" : undefined,
                                            }}
                                        >
                                            <Text size="sm" lineClamp={1}>
                                                {modelId}
                                            </Text>
                                        </Box>
                                    );
                                })
                            )}
                        </Stack>

                        {/* Ollama */}
                        {showOllama && (
                            <Stack gap="xs">
                                <Text size="sm" fw={600}>
                                    Ollama
                                </Text>
                                {ollamaModels.length === 0 ? (
                                    <Text size="xs" c="dimmed">
                                        Выберите модели в настройках
                                    </Text>
                                ) : (
                                    ollamaModels.map((modelId) => {
                                        const selected = selectedProviderId === "ollama" && selectedModel === modelId;
                                        return (
                                            <Box
                                                key={modelId}
                                                onClick={() => handleSelect("ollama", modelId)}
                                                style={{
                                                    padding: "8px 12px",
                                                    borderRadius: "var(--mantine-radius-sm)",
                                                    cursor: "pointer",
                                                    backgroundColor: selected ? "var(--mantine-color-blue-light)" : undefined,
                                                }}
                                            >
                                                <Text size="sm" lineClamp={1}>
                                                    {modelId}
                                                </Text>
                                            </Box>
                                        );
                                    })
                                )}
                            </Stack>
                        )}

                        {/* Custom providers */}
                        {customProviders.map((p) => {
                            const models = (settings.customProviderEnabledModels ?? {})[p.id] ?? [];
                            return (
                                <Stack key={p.id} gap="xs">
                                    <Text size="sm" fw={600}>
                                        {p.name}
                                    </Text>
                                    {models.length === 0 ? (
                                        <Text size="xs" c="dimmed">
                                            Выберите модели в настройках
                                        </Text>
                                    ) : (
                                        models.map((modelId) => {
                                            const selected = selectedProviderId === p.id && selectedModel === modelId;
                                            return (
                                                <Box
                                                    key={modelId}
                                                    onClick={() => handleSelect(p.id, modelId)}
                                                    style={{
                                                        padding: "8px 12px",
                                                        borderRadius: "var(--mantine-radius-sm)",
                                                        cursor: "pointer",
                                                        backgroundColor: selected ? "var(--mantine-color-blue-light)" : undefined,
                                                    }}
                                                >
                                                    <Text size="sm" lineClamp={1}>
                                                        {modelId}
                                                    </Text>
                                                </Box>
                                            );
                                        })
                                    )}
                                </Stack>
                            );
                        })}
                    </Stack>
                </ScrollArea.Autosize>
                <Group justify="flex-end" gap="sm">
                    <Button variant="subtle" onClick={onClose}>
                        Отмена
                    </Button>
                    <Button disabled={!canConfirm} onClick={handleConfirm}>
                        Создать
                    </Button>
                </Group>
            </Stack>
        </Modal>
    );
}

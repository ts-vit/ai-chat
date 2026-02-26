import { useState } from "react";
import {
    ActionIcon,
    Button,
    Card,
    Checkbox,
    Group,
    Loader,
    Modal,
    PasswordInput,
    Stack,
    Text,
    TextInput,
    Tooltip,
} from "@mantine/core";
import { IconDownload, IconPencil, IconPlus, IconTrash } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { useChatStore } from "../../store/chatStore";
import { notify } from "../../utils/notify";
import type { CustomProvider } from "../../types";
import { ConfirmModal } from "../ConfirmModal";
import { OllamaSection } from "../OllamaSection";

export interface ProvidersSectionProps {
    apiKey: string;
    onApiKeyChange: (key: string) => void;
    managementKey: string;
    onManagementKeyChange: (key: string) => void;
    ollamaUrl: string;
    onOllamaUrlChange: (url: string) => void;
    ollamaEnabledModels: string[];
    onOllamaEnabledModelsChange: (ids: string[]) => void;
    customProviderEnabledModels: Record<string, string[]>;
    onCustomProviderEnabledModelsChange: (models: Record<string, string[]>) => void;
}

export function ProvidersSection({
    apiKey,
    onApiKeyChange,
    managementKey,
    onManagementKeyChange,
    ollamaUrl,
    onOllamaUrlChange,
    ollamaEnabledModels,
    onOllamaEnabledModelsChange,
    customProviderEnabledModels,
    onCustomProviderEnabledModelsChange,
}: ProvidersSectionProps) {
    const {
        customProviders,
        createCustomProvider,
        updateCustomProvider,
        deleteCustomProvider,
    } = useChatStore();

    const [providerModalOpen, setProviderModalOpen] = useState(false);
    const [providerEditId, setProviderEditId] = useState<string | null>(null);
    const [providerName, setProviderName] = useState("");
    const [providerBaseUrl, setProviderBaseUrl] = useState("");
    const [providerApiKey, setProviderApiKey] = useState("");
    const [deletingProviderId, setDeletingProviderId] = useState<string | null>(null);
    const [customProviderModelsList, setCustomProviderModelsList] = useState<Array<{ id: string; name: string }>>([]);
    const [customProviderModelsLoading, setCustomProviderModelsLoading] = useState(false);

    const openProviderModal = (provider?: CustomProvider) => {
        if (provider) {
            setProviderEditId(provider.id);
            setProviderName(provider.name);
            setProviderBaseUrl(provider.baseUrl);
            setProviderApiKey(provider.apiKey);
        } else {
            setProviderEditId(null);
            setProviderName("");
            setProviderBaseUrl("");
            setProviderApiKey("");
        }
        setProviderModalOpen(true);
    };

    const closeProviderModal = () => {
        setProviderModalOpen(false);
        setProviderEditId(null);
        setProviderName("");
        setProviderBaseUrl("");
        setProviderApiKey("");
        setCustomProviderModelsList([]);
    };

    const loadCustomProviderModels = async () => {
        const baseUrl = providerBaseUrl.trim();
        if (!baseUrl) return;
        setCustomProviderModelsLoading(true);
        try {
            const list = await invoke<Array<{ id: string; name: string }>>("fetch_custom_provider_models", {
                baseUrl,
                apiKey: providerApiKey,
            });
            setCustomProviderModelsList(Array.isArray(list) ? list : []);
        } catch (e) {
            notify.error(String(e));
            setCustomProviderModelsList([]);
        } finally {
            setCustomProviderModelsLoading(false);
        }
    };

    const toggleCustomProviderModel = (providerId: string, modelId: string) => {
        const arr = customProviderEnabledModels[providerId] ?? [];
        const has = arr.includes(modelId);
        const next = has ? arr.filter((id) => id !== modelId) : arr.length >= 5 ? arr : [...arr, modelId];
        onCustomProviderEnabledModelsChange({ ...customProviderEnabledModels, [providerId]: next });
    };

    const saveProviderFromModal = async () => {
        const name = providerName.trim();
        const baseUrl = providerBaseUrl.trim();
        if (!name || !baseUrl) return;
        if (providerEditId) {
            await updateCustomProvider(providerEditId, name, baseUrl, providerApiKey);
        } else {
            await createCustomProvider(name, baseUrl, providerApiKey);
        }
        closeProviderModal();
    };

    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    API
                </Text>
                <PasswordInput
                    placeholder="sk-or-..."
                    value={apiKey}
                    onChange={(e) => onApiKeyChange(e.currentTarget.value)}
                />
            </Stack>

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Management-ключ OpenRouter (для отображения баланса)
                </Text>
                <PasswordInput
                    placeholder="sk-or-..."
                    value={managementKey}
                    onChange={(e) => onManagementKeyChange(e.currentTarget.value)}
                />
                <Text size="xs" c="dimmed">
                    Создайте на openrouter.ai/settings/keys с галочкой Management key
                </Text>
            </Stack>

            <OllamaSection
                ollamaUrl={ollamaUrl}
                onOllamaUrlChange={onOllamaUrlChange}
                ollamaEnabledModels={ollamaEnabledModels}
                onOllamaEnabledModelsChange={onOllamaEnabledModelsChange}
            />

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Text size="sm" fw={500}>
                        Кастомные провайдеры
                    </Text>
                    <Button
                        size="xs"
                        variant="light"
                        leftSection={<IconPlus size={16} stroke={1.5} />}
                        onClick={() => openProviderModal()}
                    >
                        Добавить провайдер
                    </Button>
                </Group>
                {customProviders.length === 0 ? (
                    <Text size="xs" c="dimmed">
                        Нет кастомных провайдеров. Добавьте OpenAI-совместимый endpoint.
                    </Text>
                ) : (
                    <Stack gap="xs">
                        {customProviders.map((p) => (
                            <Card key={p.id} withBorder padding="sm">
                                <Group justify="space-between" wrap="nowrap" align="center">
                                    <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
                                        <Text size="sm" fw={500}>
                                            {p.name}
                                        </Text>
                                        <Text size="sm" c="dimmed" lineClamp={1}>
                                            {p.baseUrl}
                                        </Text>
                                    </Stack>
                                    <Group gap="xs" wrap="nowrap">
                                        <Tooltip label="Редактировать">
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                onClick={() => openProviderModal(p)}
                                                aria-label="Редактировать"
                                            >
                                                <IconPencil size={16} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label="Удалить">
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                color="red"
                                                onClick={() => setDeletingProviderId(p.id)}
                                                aria-label="Удалить"
                                            >
                                                <IconTrash size={16} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                </Group>
                            </Card>
                        ))}
                    </Stack>
                )}
                <Modal
                    title={providerEditId ? "Редактировать провайдер" : "Добавить провайдер"}
                    opened={providerModalOpen}
                    onClose={closeProviderModal}
                    size="sm"
                >
                    <Stack gap="sm">
                        <TextInput
                            label="Название"
                            placeholder="Название провайдера"
                            value={providerName}
                            onChange={(e) => setProviderName(e.currentTarget.value)}
                            withAsterisk
                        />
                        <TextInput
                            label="Base URL"
                            placeholder="https://api.anthropic.com/v1"
                            value={providerBaseUrl}
                            onChange={(e) => setProviderBaseUrl(e.currentTarget.value)}
                            withAsterisk
                        />
                        <Text size="xs" c="dimmed">
                            Примеры: https://api.anthropic.com/v1 · https://api.groq.com/openai/v1 · https://generativelanguage.googleapis.com/v1beta/openai
                        </Text>
                        <PasswordInput
                            label="API Key"
                            placeholder="sk-..."
                            value={providerApiKey}
                            onChange={(e) => setProviderApiKey(e.currentTarget.value)}
                            description="Необязательно (для Ollama оставьте пустым)"
                        />
                        {providerEditId && (
                            <Stack gap="xs">
                                <Group justify="space-between">
                                    <Text size="sm" fw={500}>
                                        Модели (показывать при создании чата)
                                    </Text>
                                    <Text size="xs" c="dimmed">
                                        Выбрано: {(customProviderEnabledModels[providerEditId] ?? []).length}/5
                                    </Text>
                                </Group>
                                <Button
                                    size="xs"
                                    variant="light"
                                    leftSection={customProviderModelsLoading ? <Loader size={14} /> : <IconDownload size={16} stroke={1.5} />}
                                    onClick={loadCustomProviderModels}
                                    disabled={!providerBaseUrl.trim() || customProviderModelsLoading}
                                >
                                    Загрузить модели
                                </Button>
                                {customProviderModelsList.length > 0 && (
                                    <Stack gap={4}>
                                        {customProviderModelsList.map((m) => {
                                            const enabled = customProviderEnabledModels[providerEditId] ?? [];
                                            const checked = enabled.includes(m.id);
                                            const disabled = !checked && enabled.length >= 5;
                                            return (
                                                <Checkbox
                                                    key={m.id}
                                                    checked={checked}
                                                    disabled={disabled}
                                                    onChange={() => toggleCustomProviderModel(providerEditId, m.id)}
                                                    label={m.name || m.id}
                                                    size="xs"
                                                />
                                            );
                                        })}
                                    </Stack>
                                )}
                            </Stack>
                        )}
                        <Group justify="flex-end" gap="sm">
                            <Button variant="subtle" onClick={closeProviderModal}>
                                Отмена
                            </Button>
                            <Button
                                onClick={saveProviderFromModal}
                                disabled={!providerName.trim() || !providerBaseUrl.trim()}
                            >
                                Сохранить
                            </Button>
                        </Group>
                    </Stack>
                </Modal>
                <ConfirmModal
                    opened={deletingProviderId !== null}
                    onClose={() => setDeletingProviderId(null)}
                    onConfirm={() => {
                        if (deletingProviderId) {
                            deleteCustomProvider(deletingProviderId);
                            setDeletingProviderId(null);
                        }
                    }}
                    message={
                        deletingProviderId
                            ? `Удалить провайдер ${customProviders.find((x) => x.id === deletingProviderId)?.name ?? ""}?`
                            : ""
                    }
                />
            </Stack>
        </Stack>
    );
}

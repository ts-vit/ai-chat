import { useState } from "react";
import { useTranslation } from "react-i18next";
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

export interface CustomProvidersSectionProps {
    customProviderEnabledModels: Record<string, string[]>;
    onCustomProviderEnabledModelsChange: (models: Record<string, string[]>) => void;
}

export function CustomProvidersSection({
    customProviderEnabledModels,
    onCustomProviderEnabledModelsChange,
}: CustomProvidersSectionProps) {
    const { t } = useTranslation();
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
    const [providerNameError, setProviderNameError] = useState<string | null>(null);
    const [providerBaseUrlError, setProviderBaseUrlError] = useState<string | null>(null);
    const [deletingProviderId, setDeletingProviderId] = useState<string | null>(null);
    const [deletingProviderChatCount, setDeletingProviderChatCount] = useState(0);
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
        setProviderNameError(null);
        setProviderBaseUrlError(null);
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
        let hasError = false;
        if (!name) {
            setProviderNameError(t("common.fieldRequired"));
            hasError = true;
        } else {
            setProviderNameError(null);
        }
        if (!baseUrl) {
            setProviderBaseUrlError(t("common.fieldRequired"));
            hasError = true;
        } else {
            setProviderBaseUrlError(null);
        }
        if (hasError) return;
        if (providerEditId) {
            await updateCustomProvider(providerEditId, name, baseUrl, providerApiKey);
        } else {
            await createCustomProvider(name, baseUrl, providerApiKey);
        }
        closeProviderModal();
    };

    return (
        <Stack gap="lg">
            <Group justify="space-between" align="center">
                <Text size="sm" fw={500}>
                    {t("settings.customProviders.title")}
                </Text>
                <Button
                    size="xs"
                    variant="light"
                    leftSection={<IconPlus size={16} stroke={1.5} />}
                    onClick={() => openProviderModal()}
                >
                    {t("settings.customProviders.addProvider")}
                </Button>
            </Group>
            {customProviders.length === 0 ? (
                <Text size="xs" c="dimmed">
                    {t("settings.customProviders.noProviders")}
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
                                    <Tooltip label={t("common.edit")}>
                                        <ActionIcon
                                            variant="subtle"
                                            size="xs"
                                            onClick={() => openProviderModal(p)}
                                            aria-label={t("common.edit")}
                                        >
                                            <IconPencil size={16} stroke={1.5} />
                                        </ActionIcon>
                                    </Tooltip>
                                    <Tooltip label={t("common.delete")}>
                                        <ActionIcon
                                            variant="subtle"
                                            size="xs"
                                            color="red"
                                            onClick={async () => {
                                                try {
                                                    const count = await invoke<number>("get_provider_chat_count", { providerId: p.id });
                                                    setDeletingProviderChatCount(count);
                                                } catch {
                                                    setDeletingProviderChatCount(0);
                                                }
                                                setDeletingProviderId(p.id);
                                            }}
                                            aria-label={t("common.delete")}
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
                title={providerEditId ? t("settings.customProviders.editProvider") : t("settings.customProviders.addProviderModal")}
                opened={providerModalOpen}
                onClose={closeProviderModal}
                size="sm"
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("common.name")}
                        placeholder={t("settings.customProviders.namePlaceholder")}
                        value={providerName}
                        onChange={(e) => {
                            setProviderName(e.currentTarget.value);
                            setProviderNameError(null);
                        }}
                        error={providerNameError}
                        withAsterisk
                    />
                    <TextInput
                        label={t("settings.customProviders.baseUrl")}
                        placeholder="https://api.anthropic.com/v1"
                        value={providerBaseUrl}
                        onChange={(e) => {
                            setProviderBaseUrl(e.currentTarget.value);
                            setProviderBaseUrlError(null);
                        }}
                        error={providerBaseUrlError}
                        withAsterisk
                    />
                    <Text size="xs" c="dimmed">
                        {t("settings.customProviders.baseUrlExamples")}
                    </Text>
                    <PasswordInput
                        label={t("settings.customProviders.apiKey")}
                        placeholder="sk-..."
                        value={providerApiKey}
                        onChange={(e) => setProviderApiKey(e.currentTarget.value)}
                        description={t("settings.customProviders.apiKeyOptional")}
                    />
                    {providerEditId && (
                        <Stack gap="xs">
                            <Group justify="space-between">
                                <Text size="sm" fw={500}>
                                    {t("settings.customProviders.modelsTitle")}
                                </Text>
                                <Text size="xs" c="dimmed">
                                    {t("settings.openrouter.selectedCount", { count: (customProviderEnabledModels[providerEditId] ?? []).length })}
                                </Text>
                            </Group>
                            <Button
                                size="xs"
                                variant="light"
                                leftSection={customProviderModelsLoading ? <Loader size={14} /> : <IconDownload size={16} stroke={1.5} />}
                                onClick={loadCustomProviderModels}
                                disabled={!providerBaseUrl.trim() || customProviderModelsLoading}
                            >
                                {t("settings.customProviders.loadModels")}
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
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={saveProviderFromModal}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>
            <ConfirmModal
                opened={deletingProviderId !== null}
                onClose={() => setDeletingProviderId(null)}
                onConfirm={async () => {
                    if (deletingProviderId) {
                        const count = deletingProviderChatCount;
                        await deleteCustomProvider(deletingProviderId);
                        if (count > 0) {
                            notify.info(t("notifications.providerDeletedWithChats", { count }));
                        }
                        setDeletingProviderId(null);
                        setDeletingProviderChatCount(0);
                    }
                }}
                message={
                    deletingProviderId
                        ? deletingProviderChatCount > 0
                            ? t("confirm.deleteProviderWithChats", {
                                name: customProviders.find((x) => x.id === deletingProviderId)?.name ?? "",
                                count: deletingProviderChatCount,
                            })
                            : t("confirm.deleteProvider", { name: customProviders.find((x) => x.id === deletingProviderId)?.name ?? "" })
                        : ""
                }
            />
        </Stack>
    );
}

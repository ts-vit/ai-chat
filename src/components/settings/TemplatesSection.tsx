import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Accordion,
    ActionIcon,
    Box,
    Button,
    Card,
    Checkbox,
    Group,
    Modal,
    NumberInput,
    Select,
    Slider,
    Stack,
    Text,
    Textarea,
    TextInput,
    ColorSwatch,
    Tooltip,
} from "@mantine/core";
import { IconPencil, IconPlus, IconTrash } from "@tabler/icons-react";
import { FOLDER_COLORS } from "../../constants/folderColors";
import { useChatStore } from "../../store/chatStore";
import type { ChatTemplate } from "../../types";
import { ConfirmModal } from "../ConfirmModal";

function getTemplateColor(tmpl: ChatTemplate): string {
    const c = tmpl.color;
    return c && FOLDER_COLORS.includes(c as (typeof FOLDER_COLORS)[number]) ? c : "blue";
}

function getOpenRouterDisplayName(modelId: string): string {
    const lastSlash = modelId.lastIndexOf("/");
    return lastSlash >= 0 ? modelId.slice(lastSlash + 1) : modelId;
}

function formatTemplateParamsLine(t: ChatTemplate): string {
    const parts: string[] = [];
    if (t.temperature != null) parts.push(`temp: ${t.temperature}`);
    if (t.topP != null) parts.push(`top_p: ${t.topP}`);
    if (t.maxTokens != null) parts.push(`max_tokens: ${t.maxTokens}`);
    if (t.topK != null) parts.push(`top_k: ${t.topK}`);
    if (t.frequencyPenalty != null) parts.push(`freq: ${t.frequencyPenalty}`);
    if (t.presencePenalty != null) parts.push(`pres: ${t.presencePenalty}`);
    return parts.join(" · ");
}

export function TemplatesSection() {
    const { t } = useTranslation();
    const {
        templates,
        loadTemplates,
        createTemplate,
        updateTemplate,
        deleteTemplate,
        settings,
        customProviders,
        ollamaStatus,
    } = useChatStore();

    const [editingTemplate, setEditingTemplate] = useState<ChatTemplate | null>(null);
    const [isCreating, setIsCreating] = useState(false);
    const [deleteTarget, setDeleteTarget] = useState<ChatTemplate | null>(null);

    // Form state
    const [name, setName] = useState("");
    const [nameError, setNameError] = useState<string | null>(null);
    const [providerError, setProviderError] = useState<string | null>(null);
    const [modelError, setModelError] = useState<string | null>(null);
    const [icon, setIcon] = useState("");
    const [color, setColor] = useState<string>("blue");
    const [selectedProvider, setSelectedProvider] = useState<string>("");
    const [selectedModel, setSelectedModel] = useState("");
    const [systemPrompt, setSystemPrompt] = useState("");
    const [temperature, setTemperature] = useState(0.7);
    const [temperatureEnabled, setTemperatureEnabled] = useState(false);
    const [maxTokens, setMaxTokens] = useState(4096);
    const [maxTokensEnabled, setMaxTokensEnabled] = useState(false);
    const [topP, setTopP] = useState(0.9);
    const [topPEnabled, setTopPEnabled] = useState(false);
    const [topK, setTopK] = useState(40);
    const [topKEnabled, setTopKEnabled] = useState(false);
    const [frequencyPenalty, setFrequencyPenalty] = useState(0);
    const [frequencyPenaltyEnabled, setFrequencyPenaltyEnabled] = useState(false);
    const [presencePenalty, setPresencePenalty] = useState(0);
    const [presencePenaltyEnabled, setPresencePenaltyEnabled] = useState(false);

    useEffect(() => {
        loadTemplates();
    }, [loadTemplates]);

    const providerOptions = useMemo(() => {
        const opts: { value: string; label: string }[] = [
            { value: "openrouter", label: "OpenRouter" },
        ];
        if (ollamaStatus === "available") {
            opts.push({ value: "ollama", label: "Ollama" });
        }
        customProviders.forEach((p) => {
            opts.push({ value: p.id, label: p.name });
        });
        return opts;
    }, [ollamaStatus, customProviders]);

    const modelOptions = useMemo(() => {
        if (!selectedProvider) return [];
        if (selectedProvider === "openrouter") {
            const list = settings.openrouterEnabledModels ?? [];
            return list.map((id) => ({
                value: id,
                label: getOpenRouterDisplayName(id),
            }));
        }
        if (selectedProvider === "ollama") {
            const list = settings.ollamaEnabledModels ?? [];
            return list.map((id) => ({ value: id, label: id }));
        }
        const list = settings.customProviderEnabledModels?.[selectedProvider] ?? [];
        return list.map((id) => ({ value: id, label: id }));
    }, [selectedProvider, settings.openrouterEnabledModels, settings.ollamaEnabledModels, settings.customProviderEnabledModels]);

    const getProviderLabel = (providerId: string): string => {
        if (providerId === "openrouter") return "OpenRouter";
        if (providerId === "ollama") return "Ollama";
        const p = customProviders.find((x) => x.id === providerId);
        return p?.name ?? providerId;
    };

    const isFormOpen = isCreating || !!editingTemplate;

    const openForCreate = () => {
        setIsCreating(true);
        setEditingTemplate(null);
        setName("");
        setNameError(null);
        setProviderError(null);
        setModelError(null);
        setIcon("💬");
        setColor("blue");
        setSelectedProvider(providerOptions[0]?.value ?? "openrouter");
        setSelectedModel("");
        setSystemPrompt("");
        setTemperature(0.7);
        setTemperatureEnabled(false);
        setMaxTokens(4096);
        setMaxTokensEnabled(false);
        setTopP(0.9);
        setTopPEnabled(false);
        setTopK(40);
        setTopKEnabled(false);
        setFrequencyPenalty(0);
        setFrequencyPenaltyEnabled(false);
        setPresencePenalty(0);
        setPresencePenaltyEnabled(false);
    };

    const openForEdit = (tmpl: ChatTemplate) => {
        setEditingTemplate(tmpl);
        setIsCreating(false);
        setName(tmpl.name);
        setNameError(null);
        setProviderError(null);
        setModelError(null);
        setIcon(tmpl.icon || "💬");
        setColor(getTemplateColor(tmpl));
        setSelectedProvider(tmpl.providerId);
        setSelectedModel(tmpl.model);
        setSystemPrompt(tmpl.systemPrompt ?? "");
        setTemperature(tmpl.temperature ?? 0.7);
        setTemperatureEnabled(tmpl.temperature != null);
        setMaxTokens(tmpl.maxTokens ?? 4096);
        setMaxTokensEnabled(tmpl.maxTokens != null);
        setTopP(tmpl.topP ?? 0.9);
        setTopPEnabled(tmpl.topP != null);
        setTopK(tmpl.topK ?? 40);
        setTopKEnabled(tmpl.topK != null);
        setFrequencyPenalty(tmpl.frequencyPenalty ?? 0);
        setFrequencyPenaltyEnabled(tmpl.frequencyPenalty != null);
        setPresencePenalty(tmpl.presencePenalty ?? 0);
        setPresencePenaltyEnabled(tmpl.presencePenalty != null);
    };

    const closeForm = () => {
        setIsCreating(false);
        setEditingTemplate(null);
    };

    const handleProviderChange = (value: string | null) => {
        setSelectedProvider(value ?? "");
        setSelectedModel("");
    };

    const optParam = (enabled: boolean, value: number) => (enabled ? value : null);

    const handleSave = async () => {
        const trimmedName = name.trim();
        let hasError = false;
        if (!trimmedName) {
            setNameError(t("common.fieldRequired"));
            hasError = true;
        } else {
            setNameError(null);
        }
        if (!selectedProvider) {
            setProviderError(t("common.fieldRequired"));
            hasError = true;
        } else {
            setProviderError(null);
        }
        if (!selectedModel) {
            setModelError(t("common.fieldRequired"));
            hasError = true;
        } else {
            setModelError(null);
        }
        if (hasError) return;

        const params = {
            name: trimmedName,
            icon: icon.trim() || "💬",
            providerId: selectedProvider,
            model: selectedModel,
            systemPrompt: systemPrompt.trim(),
            temperature: optParam(temperatureEnabled, temperature),
            maxTokens: optParam(maxTokensEnabled, maxTokens),
            topP: optParam(topPEnabled, topP),
            topK: optParam(topKEnabled, topK),
            frequencyPenalty: optParam(frequencyPenaltyEnabled, frequencyPenalty),
            presencePenalty: optParam(presencePenaltyEnabled, presencePenalty),
        };

        if (editingTemplate) {
            await updateTemplate(editingTemplate.id, params);
        } else {
            await createTemplate(params);
        }
        closeForm();
    };

    const sortedTemplates = useMemo(
        () => [...templates].sort((a, b) => a.sortOrder - b.sortOrder),
        [templates]
    );

    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("templates.title")}
                </Text>
                <Text size="xs" c="dimmed">
                    {t("templates.description")}
                </Text>
                {sortedTemplates.length > 0 && (
                    <Group justify="space-between" align="center">
                        <div />
                        <Button
                            size="xs"
                            variant="light"
                            leftSection={<IconPlus size={18} stroke={1.5} />}
                            onClick={openForCreate}
                        >
                            {t("templates.create")}
                        </Button>
                    </Group>
                )}
            </Stack>

            {sortedTemplates.length === 0 ? (
                <Stack gap="sm">
                    <Text size="xs" c="dimmed">
                        {t("templates.noTemplates")}
                    </Text>
                    <Text size="xs" c="dimmed">
                        {t("templates.noTemplatesHint")}
                    </Text>
                    <Button
                        size="xs"
                        variant="subtle"
                        leftSection={<IconPlus size={18} stroke={1.5} />}
                        onClick={openForCreate}
                    >
                        {t("templates.create")}
                    </Button>
                </Stack>
            ) : (
                <Stack gap="xs">
                    {sortedTemplates.map((tmpl) => (
                        <Card key={tmpl.id} withBorder padding="sm">
                            <Group justify="space-between" wrap="nowrap" align="flex-start">
                                <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
                                    <Group gap="xs" wrap="nowrap">
                                        <Box
                                            style={{
                                                width: 28,
                                                height: 28,
                                                borderRadius: "50%",
                                                backgroundColor: `var(--mantine-color-${getTemplateColor(tmpl)}-5)`,
                                                display: "flex",
                                                alignItems: "center",
                                                justifyContent: "center",
                                                flexShrink: 0,
                                            }}
                                        >
                                            <Text span size="sm">
                                                {tmpl.icon || "💬"}
                                            </Text>
                                        </Box>
                                        <Text size="sm" fw={600}>
                                            {tmpl.name}
                                        </Text>
                                    </Group>
                                    <Text size="xs" c="dimmed">
                                        {t("templates.providerModel", {
                                            provider: getProviderLabel(tmpl.providerId),
                                            model:
                                                tmpl.providerId === "openrouter"
                                                    ? getOpenRouterDisplayName(tmpl.model)
                                                    : tmpl.model,
                                        })}
                                    </Text>
                                    {formatTemplateParamsLine(tmpl) && (
                                        <Text size="xs" c="dimmed">
                                            {formatTemplateParamsLine(tmpl)}
                                        </Text>
                                    )}
                                </Stack>
                                <Group gap="xs" wrap="nowrap">
                                    <Tooltip label={t("common.edit")}>
                                        <ActionIcon
                                            variant="subtle"
                                            size="xs"
                                            onClick={() => openForEdit(tmpl)}
                                            aria-label={t("common.edit")}
                                        >
                                            <IconPencil size={14} stroke={1.5} />
                                        </ActionIcon>
                                    </Tooltip>
                                    <Tooltip label={t("common.delete")}>
                                        <ActionIcon
                                            variant="subtle"
                                            size="xs"
                                            color="red"
                                            onClick={() => setDeleteTarget(tmpl)}
                                            aria-label={t("common.delete")}
                                        >
                                            <IconTrash size={14} stroke={1.5} />
                                        </ActionIcon>
                                    </Tooltip>
                                </Group>
                            </Group>
                        </Card>
                    ))}
                </Stack>
            )}

            <Modal
                title={editingTemplate ? t("templates.edit") : t("templates.create")}
                opened={isFormOpen}
                onClose={closeForm}
                size="lg"
            >
                <Stack gap="lg">
                    <TextInput
                        label={t("templates.name")}
                        placeholder={t("templates.namePlaceholder")}
                        value={name}
                        onChange={(e) => {
                            setName(e.currentTarget.value);
                            setNameError(null);
                        }}
                        error={nameError}
                        required
                    />
                    <TextInput
                        label={t("templates.icon")}
                        placeholder={t("templates.iconPlaceholder")}
                        value={icon}
                        onChange={(e) => setIcon(e.currentTarget.value)}
                        maxLength={4}
                    />
                    <Box>
                        <Text size="sm" fw={500} mb="xs" component="label">
                            {t("sidebar.folderColor")}
                        </Text>
                        <Group gap={4}>
                            {FOLDER_COLORS.map((c) => (
                                <ColorSwatch
                                    key={c}
                                    color={`var(--mantine-color-${c}-5)`}
                                    size={16}
                                    onClick={() => setColor(c)}
                                    style={{
                                        cursor: "pointer",
                                        border:
                                            color === c
                                                ? "2px solid var(--mantine-color-default-border)"
                                                : undefined,
                                    }}
                                />
                            ))}
                        </Group>
                    </Box>
                    <Select
                        label={t("templates.provider")}
                        data={providerOptions}
                        value={selectedProvider}
                        onChange={(v) => {
                            handleProviderChange(v);
                            setProviderError(null);
                        }}
                        allowDeselect={false}
                        error={providerError}
                    />
                    <Select
                        label={t("templates.model")}
                        data={modelOptions}
                        value={selectedModel}
                        onChange={(v) => {
                            setSelectedModel(v ?? "");
                            setModelError(null);
                        }}
                        allowDeselect={false}
                        disabled={modelOptions.length === 0}
                        error={modelError}
                    />
                    <Textarea
                        label={t("templates.systemPrompt")}
                        placeholder={t("templates.systemPromptPlaceholder")}
                        value={systemPrompt}
                        onChange={(e) => setSystemPrompt(e.currentTarget.value)}
                        minRows={3}
                        maxRows={8}
                        autosize
                    />

                    <Accordion>
                        <Accordion.Item value="params">
                            <Accordion.Control>
                                {t("templates.generationParams")}
                            </Accordion.Control>
                            <Accordion.Panel>
                                <Text size="xs" c="dimmed" mb="sm">
                                    {t("templates.generationParamsHint")}
                                </Text>
                                <Stack gap="md">
                                    <Stack gap="xs">
                                        <Checkbox
                                            label={t("settings.generation.temperatureWithValue", {
                                                value: temperature,
                                            })}
                                            checked={temperatureEnabled}
                                            onChange={(e) =>
                                                setTemperatureEnabled(e.currentTarget.checked)
                                            }
                                        />
                                        <Group gap="sm" wrap="nowrap">
                                            <Slider
                                                min={0}
                                                max={2}
                                                step={0.1}
                                                value={temperature}
                                                onChange={setTemperature}
                                                disabled={!temperatureEnabled}
                                                style={{ flex: 1 }}
                                            />
                                            <NumberInput
                                                value={temperature}
                                                onChange={(v) =>
                                                    setTemperature(Number(v) ?? 0.7)
                                                }
                                                min={0}
                                                max={2}
                                                step={0.1}
                                                disabled={!temperatureEnabled}
                                                style={{ width: 80 }}
                                            />
                                        </Group>
                                    </Stack>
                                    <Stack gap="xs">
                                        <Checkbox
                                            label={t("settings.generation.maxTokens")}
                                            checked={maxTokensEnabled}
                                            onChange={(e) =>
                                                setMaxTokensEnabled(e.currentTarget.checked)
                                            }
                                        />
                                        <NumberInput
                                            value={maxTokens}
                                            onChange={(v) =>
                                                setMaxTokens(Number(v) || 4096)
                                            }
                                            min={1}
                                            max={100000}
                                            disabled={!maxTokensEnabled}
                                        />
                                    </Stack>
                                    <Stack gap="xs">
                                        <Checkbox
                                            label={t("settings.generation.topP")}
                                            checked={topPEnabled}
                                            onChange={(e) =>
                                                setTopPEnabled(e.currentTarget.checked)
                                            }
                                        />
                                        <Group gap="sm" wrap="nowrap">
                                            <Slider
                                                min={0}
                                                max={1}
                                                step={0.01}
                                                value={topP}
                                                onChange={setTopP}
                                                disabled={!topPEnabled}
                                                style={{ flex: 1 }}
                                            />
                                            <NumberInput
                                                value={topP}
                                                onChange={(v) =>
                                                    setTopP(Number(v) ?? 0.9)
                                                }
                                                min={0}
                                                max={1}
                                                step={0.01}
                                                disabled={!topPEnabled}
                                                style={{ width: 80 }}
                                            />
                                        </Group>
                                    </Stack>
                                    <Stack gap="xs">
                                        <Checkbox
                                            label={t("settings.generation.topK")}
                                            checked={topKEnabled}
                                            onChange={(e) =>
                                                setTopKEnabled(e.currentTarget.checked)
                                            }
                                        />
                                        <NumberInput
                                            value={topK}
                                            onChange={(v) =>
                                                setTopK(Number(v) || 40)
                                            }
                                            min={1}
                                            max={500}
                                            disabled={!topKEnabled}
                                        />
                                    </Stack>
                                    <Stack gap="xs">
                                        <Checkbox
                                            label={t("settings.generation.frequencyPenalty")}
                                            checked={frequencyPenaltyEnabled}
                                            onChange={(e) =>
                                                setFrequencyPenaltyEnabled(
                                                    e.currentTarget.checked
                                                )
                                            }
                                        />
                                        <Group gap="sm" wrap="nowrap">
                                            <Slider
                                                min={-2}
                                                max={2}
                                                step={0.1}
                                                value={frequencyPenalty}
                                                onChange={setFrequencyPenalty}
                                                disabled={!frequencyPenaltyEnabled}
                                                style={{ flex: 1 }}
                                            />
                                            <NumberInput
                                                value={frequencyPenalty}
                                                onChange={(v) =>
                                                    setFrequencyPenalty(
                                                        Number(v) ?? 0
                                                    )
                                                }
                                                min={-2}
                                                max={2}
                                                step={0.1}
                                                disabled={!frequencyPenaltyEnabled}
                                                style={{ width: 80 }}
                                            />
                                        </Group>
                                    </Stack>
                                    <Stack gap="xs">
                                        <Checkbox
                                            label={t("settings.generation.presencePenalty")}
                                            checked={presencePenaltyEnabled}
                                            onChange={(e) =>
                                                setPresencePenaltyEnabled(
                                                    e.currentTarget.checked
                                                )
                                            }
                                        />
                                        <Group gap="sm" wrap="nowrap">
                                            <Slider
                                                min={-2}
                                                max={2}
                                                step={0.1}
                                                value={presencePenalty}
                                                onChange={setPresencePenalty}
                                                disabled={!presencePenaltyEnabled}
                                                style={{ flex: 1 }}
                                            />
                                            <NumberInput
                                                value={presencePenalty}
                                                onChange={(v) =>
                                                    setPresencePenalty(
                                                        Number(v) ?? 0
                                                    )
                                                }
                                                min={-2}
                                                max={2}
                                                step={0.1}
                                                disabled={!presencePenaltyEnabled}
                                                style={{ width: 80 }}
                                            />
                                        </Group>
                                    </Stack>
                                </Stack>
                            </Accordion.Panel>
                        </Accordion.Item>
                    </Accordion>

                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={closeForm}>
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={handleSave}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            <ConfirmModal
                opened={!!deleteTarget}
                onClose={() => setDeleteTarget(null)}
                onConfirm={async () => {
                    if (deleteTarget) {
                        await deleteTemplate(deleteTarget.id);
                        setDeleteTarget(null);
                    }
                }}
                title={
                    deleteTarget
                        ? t("templates.deleteConfirm", { name: deleteTarget.name })
                        : undefined
                }
                message={t("confirm.cannotUndo")}
            />
        </Stack>
    );
}

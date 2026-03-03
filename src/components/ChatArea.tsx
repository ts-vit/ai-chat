import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionIcon, Box, Button, Group, Menu, Modal, Popover, Select, Stack, Text, Textarea, Title, Tooltip } from "@mantine/core";
import {
    IconAdjustments,
    IconDownload,
    IconLayoutSidebarLeftCollapse,
    IconLayoutSidebarLeftExpand,
    IconLayoutSidebarRightCollapse,
    IconLayoutSidebarRightExpand,
    IconMessage2,
    IconMessageChatbot,
    IconPlus,
} from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { MessageList } from "./MessageList";
import { MessageInput } from "./MessageInput";
import { ChatStats } from "./ChatStats";
import { ChatParamsPopoverContent } from "./ChatParamsPopover";
import { useChatStore } from "../store/chatStore";
import { notify } from "../utils/notify";

const CUSTOM_PROMPT_VALUE = "__custom__";

interface ChatAreaProps {
    onToggleLeftSidebar: () => void;
    onToggleRightSidebar: () => void;
    leftSidebarOpen: boolean;
    rightSidebarOpen: boolean;
    compact?: boolean;
    hideStats?: boolean;
    onNewChat: () => void;
    messageInputRef?: React.RefObject<HTMLTextAreaElement | null>;
}

export function ChatArea({
    onToggleLeftSidebar,
    onToggleRightSidebar,
    leftSidebarOpen,
    rightSidebarOpen,
    compact = false,
    hideStats = false,
    onNewChat,
    messageInputRef,
}: ChatAreaProps) {
    const { t } = useTranslation();
    const {
        chats,
        activeChatId,
        presets,
        models,
        isStreaming,
        isStopping,
        sendMessage,
        editAndResend,
        stopGeneration,
        setChatSystemPrompt,
        updateChatModel,
        updateChatParams,
        settings,
        customProviders,
        localOllamaModels,
        loadLocalOllamaModels,
    } = useChatStore();

    const activeChat = chats.find((c) => c.id === activeChatId);
    const [customModalOpen, setCustomModalOpen] = useState(false);
    const [customPromptDraft, setCustomPromptDraft] = useState("");
    const [presetPopoverOpen, setPresetPopoverOpen] = useState(false);
    const [paramsPopoverOpen, setParamsPopoverOpen] = useState(false);
    const [customProviderModelsCache, setCustomProviderModelsCache] = useState<Record<string, string[]>>({});
    const [customModelsLoading, setCustomModelsLoading] = useState(false);

    const presetSelectData = [
        { value: "", label: t("chat.noPrompt") },
        ...presets.map((p) => ({ value: p.id, label: p.name })),
        { value: CUSTOM_PROMPT_VALUE, label: t("chat.customPrompt") },
    ];

    const currentSystemPrompt = activeChat?.systemPrompt ?? "";
    const matchingPreset = currentSystemPrompt
        ? presets.find((p) => p.content === currentSystemPrompt)
        : null;
    const selectValue = customModalOpen
        ? CUSTOM_PROMPT_VALUE
        : matchingPreset
          ? matchingPreset.id
          : currentSystemPrompt
            ? CUSTOM_PROMPT_VALUE
            : "";

    const providerId = activeChat?.providerId ?? "openrouter";
    const isCustomProvider = providerId !== "openrouter" && providerId !== "ollama";

    useEffect(() => {
        if (providerId === "ollama") {
            loadLocalOllamaModels();
        }
    }, [providerId, loadLocalOllamaModels]);

    useEffect(() => {
        if (!isCustomProvider || !providerId) return;
        if (customProviderModelsCache[providerId] !== undefined) return;
        const provider = customProviders.find((p) => p.id === providerId);
        if (!provider) return;
        setCustomModelsLoading(true);
        invoke<Array<{ id: string; name: string }>>("fetch_custom_provider_models", {
            baseUrl: provider.baseUrl,
            apiKey: provider.apiKey ?? "",
        })
            .then((list) => {
                const ids = Array.isArray(list) ? list.map((m) => m.id) : [];
                setCustomProviderModelsCache((prev) => ({ ...prev, [providerId]: ids }));
            })
            .catch((e) => {
                notify.error(String(e));
                setCustomProviderModelsCache((prev) => ({ ...prev, [providerId]: [] }));
            })
            .finally(() => setCustomModelsLoading(false));
    }, [isCustomProvider, providerId, customProviders, customProviderModelsCache]);

    const modelSelectData = useMemo(() => {
        if (providerId === "openrouter") {
            const models = settings.openrouterEnabledModels ?? [];
            return models.map((id) => ({ value: id, label: id }));
        }
        if (providerId === "ollama") {
            return localOllamaModels.map((m) => ({ value: m.name, label: m.name }));
        }
        if (isCustomProvider) {
            const ids = customProviderModelsCache[providerId] ?? [];
            return ids.map((id) => ({ value: id, label: id }));
        }
        return [];
    }, [providerId, isCustomProvider, settings.openrouterEnabledModels, localOllamaModels, customProviderModelsCache]);

    const handleModelChange = (value: string | null) => {
        if (!activeChatId || value === null || value === "") return;
        const modelInfo = models.find((m) => m.id === value);
        updateChatModel(activeChatId, value, modelInfo?.supportsImageGeneration ?? false);
    };

    const handlePresetChange = (value: string | null) => {
        if (!activeChatId) return;
        setPresetPopoverOpen(false);
        if (value === null || value === "") {
            setChatSystemPrompt(activeChatId, "");
            return;
        }
        if (value === CUSTOM_PROMPT_VALUE) {
            setCustomPromptDraft(currentSystemPrompt);
            setCustomModalOpen(true);
            return;
        }
        const preset = presets.find((p) => p.id === value);
        if (preset) setChatSystemPrompt(activeChatId, preset.content);
    };

    const saveCustomPrompt = () => {
        if (activeChatId) {
            setChatSystemPrompt(activeChatId, customPromptDraft);
        }
        setCustomModalOpen(false);
    };

    return (
        <Box
            style={{
                flex: 1,
                minWidth: 0,
                overflow: "hidden",
                display: "flex",
                flexDirection: "column",
                height: "100vh",
            }}
        >
            <Group
                p="xs"
                gap="xs"
                style={{
                    height: 40,
                    flexShrink: 0,
                    borderBottom: "1px solid var(--mantine-color-default-border)",
                }}
                justify="space-between"
                align="center"
            >
                <Tooltip label={leftSidebarOpen ? t("chat.hideLeftPanel") : t("chat.showLeftPanel")}>
                    <ActionIcon
                        variant="subtle"
                        size="lg"
                        onClick={onToggleLeftSidebar}
                        aria-label={leftSidebarOpen ? t("chat.hideLeftPanel") : t("chat.showLeftPanel")}
                    >
                        {leftSidebarOpen ? (
                            <IconLayoutSidebarLeftCollapse size={18} stroke={1.5} />
                        ) : (
                            <IconLayoutSidebarLeftExpand size={18} stroke={1.5} />
                        )}
                    </ActionIcon>
                </Tooltip>
                {activeChat &&
                    (compact ? (
                        <Popover
                            width={220}
                            position="bottom"
                            withArrow
                            shadow="md"
                            opened={presetPopoverOpen}
                            onChange={setPresetPopoverOpen}
                        >
                            <Popover.Target>
                                <Tooltip label={t("chat.systemPrompt")}>
                                    <ActionIcon
                                        variant="subtle"
                                        size="lg"
                                        onClick={() => setPresetPopoverOpen((o) => !o)}
                                        aria-label={t("chat.systemPrompt")}
                                    >
                                        <IconMessage2 size={18} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            </Popover.Target>
                            <Popover.Dropdown>
                                <Select
                                    size="xs"
                                    data={presetSelectData}
                                    value={selectValue}
                                    onChange={handlePresetChange}
                                    placeholder={t("chat.systemPrompt")}
                                    allowDeselect={false}
                                    styles={{ input: { minWidth: "100%" } }}
                                />
                            </Popover.Dropdown>
                        </Popover>
                    ) : (
                        <Select
                            size="xs"
                            data={presetSelectData}
                            value={selectValue}
                            onChange={handlePresetChange}
                            placeholder="Системный промпт"
                            allowDeselect={false}
                            styles={{ input: { minWidth: 120, maxWidth: 180 } }}
                        />
                    ))}
                {activeChat && (
                    <Select
                        size="xs"
                        data={modelSelectData}
                        value={activeChat.model ?? ""}
                        onChange={handleModelChange}
                        placeholder={customModelsLoading && isCustomProvider ? t("common.loading") : t("chat.selectModel")}
                        searchable
                        allowDeselect={false}
                        disabled={customModelsLoading && isCustomProvider}
                        styles={{ input: { minWidth: 140, maxWidth: 300 } }}
                    />
                )}
                {activeChat && (
                    <Menu position="bottom-end" withArrow>
                        <Menu.Target>
                            <Tooltip label={t("chat.export")}>
                                <ActionIcon variant="subtle" size="xs" aria-label={t("chat.export")}>
                                    <IconDownload size={16} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        </Menu.Target>
                        <Menu.Dropdown>
                            <Menu.Item
                                onClick={async () => {
                                    if (!activeChatId) return;
                                    try {
                                        await invoke("export_chat_json", { chatId: activeChatId });
                                        notify.success(t("chat.exportSuccess"));
                                    } catch (e) {
                                        notify.error(String(e));
                                    }
                                }}
                            >
                                {t("chat.exportJson")}
                            </Menu.Item>
                            <Menu.Item
                                onClick={async () => {
                                    if (!activeChatId) return;
                                    try {
                                        await invoke("export_chat_markdown", { chatId: activeChatId });
                                        notify.success(t("chat.exportSuccess"));
                                    } catch (e) {
                                        notify.error(String(e));
                                    }
                                }}
                            >
                                {t("chat.exportMarkdown")}
                            </Menu.Item>
                        </Menu.Dropdown>
                    </Menu>
                )}
                {activeChat && (
                    <Popover
                        width={320}
                        position="bottom-end"
                        withArrow
                        shadow="md"
                        opened={paramsPopoverOpen}
                        onChange={setParamsPopoverOpen}
                    >
                        <Popover.Target>
                            <Tooltip label={t("chatParams.title")}>
                                <ActionIcon
                                    variant="subtle"
                                    size="xs"
                                    onClick={() => setParamsPopoverOpen((o) => !o)}
                                    aria-label={t("chatParams.title")}
                                >
                                    <IconAdjustments size={16} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        </Popover.Target>
                        <Popover.Dropdown>
                            <ChatParamsPopoverContent
                                chat={activeChat}
                                settings={settings}
                                onParamsChange={updateChatParams}
                            />
                        </Popover.Dropdown>
                    </Popover>
                )}
                <Tooltip label={rightSidebarOpen ? t("chat.hideRightPanel") : t("chat.showRightPanel")}>
                    <ActionIcon
                        variant="subtle"
                        size="lg"
                        onClick={onToggleRightSidebar}
                        aria-label={rightSidebarOpen ? t("chat.hideRightPanel") : t("chat.showRightPanel")}
                    >
                        {rightSidebarOpen ? (
                            <IconLayoutSidebarRightCollapse size={18} stroke={1.5} />
                        ) : (
                            <IconLayoutSidebarRightExpand size={18} stroke={1.5} />
                        )}
                    </ActionIcon>
                </Tooltip>
            </Group>
            <Modal
                title={t("chat.customPromptModalTitle")}
                size="md"
                opened={customModalOpen}
                onClose={() => setCustomModalOpen(false)}
            >
                <Stack gap="sm">
                    <Textarea
                        placeholder={t("chat.customPromptPlaceholder")}
                        value={customPromptDraft}
                        onChange={(e) => setCustomPromptDraft(e.currentTarget.value)}
                        minRows={3}
                        maxRows={10}
                        autosize
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={() => setCustomModalOpen(false)}>
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={saveCustomPrompt}>{t("common.save")}</Button>
                    </Group>
                </Stack>
            </Modal>

            {!activeChat ? (
                <Box
                    style={{
                        flex: 1,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                    }}
                >
                    <Stack align="center" gap="md">
                        <Box c="dimmed">
                            <IconMessageChatbot size={64} stroke={1.2} />
                        </Box>
                        <Title order={3} c="dimmed">
                            {t("chat.emptyStateTitle")}
                        </Title>
                        <Text size="sm" c="dimmed" ta="center">
                            {t("chat.emptyStateHint")}
                        </Text>
                        <Button variant="light" leftSection={<IconPlus size={16} />} onClick={onNewChat}>
                            {t("chat.newChat")}
                        </Button>
                    </Stack>
                </Box>
            ) : (
                <>
                    <MessageList
                        messages={activeChat.messages}
                        isStreaming={isStreaming}
                        onEditResend={editAndResend}
                        compact={compact}
                    />
                    {!hideStats && (
                        <ChatStats
                            compact={compact}
                            providerId={activeChat?.providerId ?? "openrouter"}
                        />
                    )}
                    <MessageInput
                        onSend={sendMessage}
                        onStop={stopGeneration}
                        disabled={isStreaming}
                        isStopping={isStopping}
                        compact={compact}
                        inputRef={messageInputRef}
                    />
                </>
            )}
        </Box>
    );
}
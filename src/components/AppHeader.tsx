import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Box,
    Button,
    Group,
    Menu,
    Modal,
    Popover,
    Select,
    Stack,
    Text,
    Textarea,
    Tooltip,
} from "@mantine/core";
import {
    IconAdjustments,
    IconDotsVertical,
    IconDownload,
    IconLayoutSidebarLeftCollapse,
    IconTrash,
    IconLayoutSidebarLeftExpand,
    IconLayoutSidebarRightCollapse,
    IconLayoutSidebarRightExpand,
    IconMessage2,
} from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import logo from "../assets/sp-logo.png";
import { ChatParamsPopoverContent } from "./ChatParamsPopover";
import { useChatStore } from "../store/chatStore";
import { notify } from "../utils/notify";
import type { Chat, Comparison } from "../types";
import { ConfirmModal } from "./ConfirmModal";

const CUSTOM_PROMPT_VALUE = "__custom__";

interface AppHeaderProps {
    leftSidebarOpen: boolean;
    rightSidebarOpen: boolean;
    onToggleLeftSidebar: () => void;
    onToggleRightSidebar: () => void;
    activeChat: Chat | undefined;
    activeComparison?: Comparison | null;
    effectiveLeftWidth: number;
    isNarrow: boolean;
    isVeryNarrow: boolean;
}

export function AppHeader({
    leftSidebarOpen,
    rightSidebarOpen,
    onToggleLeftSidebar,
    onToggleRightSidebar,
    activeChat,
    activeComparison,
    effectiveLeftWidth,
    isNarrow,
    isVeryNarrow,
}: AppHeaderProps) {
    const { t } = useTranslation();
    const {
        presets,
        setChatSystemPrompt,
        updateChatModel,
        updateChatParams,
        settings,
        customProviders,
        localOllamaModels,
        loadLocalOllamaModels,
        loadMcpConnections,
        models,
        deleteComparison,
    } = useChatStore();

    const [compareDeleteConfirmOpen, setCompareDeleteConfirmOpen] = useState(false);

    const [customModalOpen, setCustomModalOpen] = useState(false);
    const [customPromptDraft, setCustomPromptDraft] = useState("");
    const [presetPopoverOpen, setPresetPopoverOpen] = useState(false);
    const [paramsPopoverOpen, setParamsPopoverOpen] = useState(false);
    const [customProviderModelsCache, setCustomProviderModelsCache] = useState<
        Record<string, string[]>
    >({});
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
    const isCustomProvider =
        providerId !== "openrouter" && providerId !== "ollama";

    useEffect(() => {
        loadMcpConnections();
    }, [loadMcpConnections]);

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
        invoke<Array<{ id: string; name: string }>>(
            "fetch_custom_provider_models",
            {
                baseUrl: provider.baseUrl,
                apiKey: provider.apiKey ?? "",
            }
        )
            .then((list) => {
                const ids = Array.isArray(list)
                    ? list.map((m) => m.id)
                    : [];
                setCustomProviderModelsCache((prev) => ({
                    ...prev,
                    [providerId]: ids,
                }));
            })
            .catch((e) => {
                notify.error(String(e));
                setCustomProviderModelsCache((prev) => ({
                    ...prev,
                    [providerId]: [],
                }));
            })
            .finally(() => setCustomModelsLoading(false));
    }, [
        isCustomProvider,
        providerId,
        customProviders,
        customProviderModelsCache,
    ]);

    const modelSelectData = useMemo(() => {
        if (providerId === "openrouter") {
            const openRouterModels = settings.openrouterEnabledModels ?? [];
            return openRouterModels.map((id) => ({ value: id, label: id }));
        }
        if (providerId === "ollama") {
            return localOllamaModels.map((m) => ({
                value: m.name,
                label: m.name,
            }));
        }
        if (isCustomProvider) {
            const ids = customProviderModelsCache[providerId] ?? [];
            return ids.map((id) => ({ value: id, label: id }));
        }
        return [];
    }, [
        providerId,
        isCustomProvider,
        settings.openrouterEnabledModels,
        localOllamaModels,
        customProviderModelsCache,
    ]);

    const handleModelChange = (value: string | null) => {
        if (!activeChat?.id || value === null || value === "") return;
        const modelInfo = models.find((m) => m.id === value);
        updateChatModel(
            activeChat.id,
            value,
            modelInfo?.supportsImageGeneration ?? false
        );
    };

    const handlePresetChange = (value: string | null) => {
        if (!activeChat?.id) return;
        setPresetPopoverOpen(false);
        if (value === null || value === "") {
            setChatSystemPrompt(activeChat.id, "");
            return;
        }
        if (value === CUSTOM_PROMPT_VALUE) {
            setCustomPromptDraft(currentSystemPrompt);
            setCustomModalOpen(true);
            return;
        }
        const preset = presets.find((p) => p.id === value);
        if (preset) setChatSystemPrompt(activeChat.id, preset.content);
    };

    const saveCustomPrompt = () => {
        if (activeChat?.id) {
            setChatSystemPrompt(activeChat.id, customPromptDraft);
        }
        setCustomModalOpen(false);
    };

    const leftZoneWidth = leftSidebarOpen ? effectiveLeftWidth : undefined;

    return (
        <Box
            className="app-header"
            style={{
                height: 48,
                flexShrink: 0,
                borderBottom:
                    "1px solid var(--mantine-color-default-border)",
                background: "var(--mantine-color-body)",
                display: "flex",
                alignItems: "center",
                width: "100%",
            }}
        >
            {/* Left zone */}
            <Group
                gap="xs"
                style={{
                    width: leftZoneWidth,
                    minWidth: leftZoneWidth ?? undefined,
                    paddingLeft: "var(--mantine-spacing-xs)",
                    paddingRight: "var(--mantine-spacing-xs)",
                    flexShrink: 0,
                }}
                wrap="nowrap"
            >
                <img
                    src={logo}
                    alt=""
                    style={{ height: 24, display: "block" }}
                />
                {!isNarrow && (
                    <Text size="sm" fw={600}>
                        {t("settings.about.appName")}
                    </Text>
                )}
                <Tooltip
                    label={
                        leftSidebarOpen
                            ? t("chat.hideLeftPanel")
                            : t("chat.showLeftPanel")
                    }
                >
                    <ActionIcon
                        variant="subtle"
                        size="lg"
                        onClick={onToggleLeftSidebar}
                        aria-label={
                            leftSidebarOpen
                                ? t("chat.hideLeftPanel")
                                : t("chat.showLeftPanel")
                        }
                    >
                        {leftSidebarOpen ? (
                            <IconLayoutSidebarLeftCollapse
                                size={18}
                                stroke={1.5}
                            />
                        ) : (
                            <IconLayoutSidebarLeftExpand
                                size={18}
                                stroke={1.5}
                            />
                        )}
                    </ActionIcon>
                </Tooltip>
            </Group>

            {/* Center zone — chat or compare */}
            {activeComparison !== undefined ? (
                <Group
                    gap="sm"
                    style={{
                        flex: 1,
                        justifyContent: "center",
                        alignItems: "center",
                        minWidth: 0,
                        paddingLeft: "var(--mantine-spacing-xs)",
                        paddingRight: "var(--mantine-spacing-xs)",
                    }}
                    wrap="nowrap"
                >
                    <Stack gap={0} style={{ flex: 1, minWidth: 0 }} align="center">
                        <Text size="sm" fw={600} lineClamp={1}>
                            {activeComparison?.title ?? t("compare.title")}
                        </Text>
                        {activeComparison && (
                            <Text size="xs" c="dimmed">
                                {activeComparison.leftModel.split("/").pop()} {t("compare.vsLabel")}{" "}
                                {activeComparison.rightModel.split("/").pop()}
                            </Text>
                        )}
                    </Stack>
                    {activeComparison && (
                        <Tooltip label={t("common.delete")}>
                            <ActionIcon
                                variant="subtle"
                                size="sm"
                                color="red"
                                onClick={() => setCompareDeleteConfirmOpen(true)}
                                aria-label={t("common.delete")}
                            >
                                <IconTrash size={16} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    )}
                </Group>
            ) : activeChat ? (
                <Group
                    gap="sm"
                    style={{
                        flex: 1,
                        justifyContent: "center",
                        alignItems: "center",
                        minWidth: 0,
                        paddingLeft: "var(--mantine-spacing-xs)",
                        paddingRight: "var(--mantine-spacing-xs)",
                    }}
                    wrap="nowrap"
                >
                    {isVeryNarrow ? (
                        <>
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
                                            onClick={() =>
                                                setPresetPopoverOpen((o) => !o)
                                            }
                                            aria-label={t("chat.systemPrompt")}
                                        >
                                            <IconMessage2
                                                size={18}
                                                stroke={1.5}
                                            />
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
                                        styles={{
                                            input: { minWidth: "100%" },
                                        }}
                                    />
                                </Popover.Dropdown>
                            </Popover>
                            <Popover
                                width={320}
                                position="bottom-end"
                                withArrow
                                shadow="md"
                                opened={paramsPopoverOpen}
                                onChange={setParamsPopoverOpen}
                            >
                                <Menu position="bottom-end" withArrow>
                                    <Menu.Target>
                                        <Popover.Target>
                                            <Tooltip label={t("header.more")}>
                                                <ActionIcon
                                                    variant="subtle"
                                                    size="lg"
                                                    aria-label={t("header.more")}
                                                >
                                                    <IconDotsVertical
                                                        size={18}
                                                        stroke={1.5}
                                                    />
                                                </ActionIcon>
                                            </Tooltip>
                                        </Popover.Target>
                                    </Menu.Target>
                                    <Menu.Dropdown>
                                        <Menu.Label>
                                            {activeChat.model ?? t("chat.selectModel")}
                                        </Menu.Label>
                                        <Box
                                            component="div"
                                            onClick={(e) => e.stopPropagation()}
                                        >
                                            <Select
                                                size="xs"
                                                data={modelSelectData}
                                                value={activeChat.model ?? ""}
                                                onChange={handleModelChange}
                                                placeholder={
                                                    customModelsLoading &&
                                                    isCustomProvider
                                                        ? t("common.loading")
                                                        : t("chat.selectModel")
                                                }
                                                searchable
                                                allowDeselect={false}
                                                disabled={
                                                    customModelsLoading &&
                                                    isCustomProvider
                                                }
                                                styles={{
                                                    input: {
                                                        minWidth: 140,
                                                        maxWidth: 260,
                                                    },
                                                }}
                                            />
                                        </Box>
                                        <Menu.Divider />
                                        <Menu.Item
                                        leftSection={
                                            <IconAdjustments
                                                size={16}
                                                stroke={1.5}
                                            />
                                        }
                                        onClick={() =>
                                            setParamsPopoverOpen(true)
                                        }
                                    >
                                        {t("chatParams.title")}
                                    </Menu.Item>
                                    <Menu.Item
                                        leftSection={
                                            <IconDownload
                                                size={16}
                                                stroke={1.5}
                                            />
                                        }
                                        onClick={async () => {
                                            if (!activeChat.id) return;
                                            try {
                                                await invoke(
                                                    "export_chat_json",
                                                    {
                                                        chatId: activeChat.id,
                                                    }
                                                );
                                                notify.success(
                                                    t("chat.exportSuccess")
                                                );
                                            } catch (e) {
                                                notify.error(String(e));
                                            }
                                        }}
                                    >
                                        {t("chat.exportJson")}
                                    </Menu.Item>
                                    <Menu.Item
                                        leftSection={
                                            <IconDownload
                                                size={16}
                                                stroke={1.5}
                                            />
                                        }
                                        onClick={async () => {
                                            if (!activeChat.id) return;
                                            try {
                                                await invoke(
                                                    "export_chat_markdown",
                                                    {
                                                        chatId: activeChat.id,
                                                    }
                                                );
                                                notify.success(
                                                    t("chat.exportSuccess")
                                                );
                                            } catch (e) {
                                                notify.error(String(e));
                                            }
                                        }}
                                    >
                                        {t("chat.exportMarkdown")}
                                    </Menu.Item>
                                    </Menu.Dropdown>
                                </Menu>
                                <Popover.Dropdown>
                                    <ChatParamsPopoverContent
                                        chat={activeChat}
                                        settings={settings}
                                        onParamsChange={updateChatParams}
                                    />
                                </Popover.Dropdown>
                            </Popover>
                        </>
                    ) : (
                        <>
                            <Select
                                size="xs"
                                data={presetSelectData}
                                value={selectValue}
                                onChange={handlePresetChange}
                                placeholder={t("chat.systemPrompt")}
                                allowDeselect={false}
                                styles={{
                                    input: { minWidth: 120, maxWidth: 180 },
                                }}
                            />
                            <Text size="xs" c="dimmed">
                                {activeChat.model ?? "—"}
                            </Text>
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
                                            size="sm"
                                            onClick={() =>
                                                setParamsPopoverOpen(
                                                    (o) => !o
                                                )
                                            }
                                            aria-label={t("chatParams.title")}
                                        >
                                            <IconAdjustments
                                                size={16}
                                                stroke={1.5}
                                            />
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
                            <Menu position="bottom-end" withArrow>
                                <Menu.Target>
                                    <Tooltip label={t("chat.export")}>
                                        <ActionIcon
                                            variant="subtle"
                                            size="sm"
                                            aria-label={t("chat.export")}
                                        >
                                            <IconDownload
                                                size={16}
                                                stroke={1.5}
                                            />
                                        </ActionIcon>
                                    </Tooltip>
                                </Menu.Target>
                                <Menu.Dropdown>
                                    <Menu.Item
                                        onClick={async () => {
                                            if (!activeChat.id) return;
                                            try {
                                                await invoke(
                                                    "export_chat_json",
                                                    {
                                                        chatId: activeChat.id,
                                                    }
                                                );
                                                notify.success(
                                                    t("chat.exportSuccess")
                                                );
                                            } catch (e) {
                                                notify.error(String(e));
                                            }
                                        }}
                                    >
                                        {t("chat.exportJson")}
                                    </Menu.Item>
                                    <Menu.Item
                                        onClick={async () => {
                                            if (!activeChat.id) return;
                                            try {
                                                await invoke(
                                                    "export_chat_markdown",
                                                    {
                                                        chatId: activeChat.id,
                                                    }
                                                );
                                                notify.success(
                                                    t("chat.exportSuccess")
                                                );
                                            } catch (e) {
                                                notify.error(String(e));
                                            }
                                        }}
                                    >
                                        {t("chat.exportMarkdown")}
                                    </Menu.Item>
                                </Menu.Dropdown>
                            </Menu>
                        </>
                    )}
                </Group>
            ) : null}

            {/* Right zone */}
            <Group
                gap="xs"
                style={{
                    paddingRight: "var(--mantine-spacing-xs)",
                    flexShrink: 0,
                }}
            >
                <Tooltip
                    label={
                        rightSidebarOpen
                            ? t("chat.hideRightPanel")
                            : t("chat.showRightPanel")
                    }
                >
                    <ActionIcon
                        variant="subtle"
                        size="lg"
                        onClick={onToggleRightSidebar}
                        aria-label={
                            rightSidebarOpen
                                ? t("chat.hideRightPanel")
                                : t("chat.showRightPanel")
                        }
                    >
                        {rightSidebarOpen ? (
                            <IconLayoutSidebarRightCollapse
                                size={18}
                                stroke={1.5}
                            />
                        ) : (
                            <IconLayoutSidebarRightExpand
                                size={18}
                                stroke={1.5}
                            />
                        )}
                    </ActionIcon>
                </Tooltip>
            </Group>

            {activeComparison !== undefined && (
                <ConfirmModal
                    opened={compareDeleteConfirmOpen}
                    onClose={() => setCompareDeleteConfirmOpen(false)}
                    onConfirm={() => {
                        if (activeComparison) {
                            deleteComparison(activeComparison.id);
                        }
                        setCompareDeleteConfirmOpen(false);
                    }}
                    message={t("compare.deleteConfirm")}
                />
            )}

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
                        onChange={(e) =>
                            setCustomPromptDraft(e.currentTarget.value)
                        }
                        minRows={3}
                        maxRows={10}
                        autosize
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button
                            variant="subtle"
                            onClick={() => setCustomModalOpen(false)}
                        >
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={saveCustomPrompt}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>
        </Box>
    );
}

import { useState } from "react";
import { ActionIcon, Badge, Box, Button, Group, Menu, Modal, Popover, Select, Stack, Text, Textarea, Title, Tooltip } from "@mantine/core";
import {
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
    const {
        chats,
        activeChatId,
        presets,
        isStreaming,
        isStopping,
        sendMessage,
        editAndResend,
        stopGeneration,
        setChatSystemPrompt,
    } = useChatStore();

    const activeChat = chats.find((c) => c.id === activeChatId);
    const [customModalOpen, setCustomModalOpen] = useState(false);
    const [customPromptDraft, setCustomPromptDraft] = useState("");
    const [presetPopoverOpen, setPresetPopoverOpen] = useState(false);

    const presetSelectData = [
        { value: "", label: "Без промпта" },
        ...presets.map((p) => ({ value: p.id, label: p.name })),
        { value: CUSTOM_PROMPT_VALUE, label: "Кастомный..." },
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
                <Tooltip label={leftSidebarOpen ? "Скрыть левую панель" : "Показать левую панель"}>
                    <ActionIcon
                        variant="subtle"
                        size="lg"
                        onClick={onToggleLeftSidebar}
                        aria-label={leftSidebarOpen ? "Скрыть левую панель" : "Показать левую панель"}
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
                                <Tooltip label="Системный промпт">
                                    <ActionIcon
                                        variant="subtle"
                                        size="lg"
                                        onClick={() => setPresetPopoverOpen((o) => !o)}
                                        aria-label="Системный промпт"
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
                                    placeholder="Системный промпт"
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
                {activeChat?.model ? (
                    <Badge variant="light" size="sm" title={activeChat.model} style={{ maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis" }}>
                        {activeChat.model}
                    </Badge>
                ) : null}
                {activeChat && (
                    <Menu position="bottom-end" withArrow>
                        <Menu.Target>
                            <Tooltip label="Экспорт">
                                <ActionIcon variant="subtle" size="xs" aria-label="Экспорт">
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
                                        notify.success("Чат экспортирован");
                                    } catch (e) {
                                        notify.error(String(e));
                                    }
                                }}
                            >
                                Экспорт в JSON
                            </Menu.Item>
                            <Menu.Item
                                onClick={async () => {
                                    if (!activeChatId) return;
                                    try {
                                        await invoke("export_chat_markdown", { chatId: activeChatId });
                                        notify.success("Чат экспортирован");
                                    } catch (e) {
                                        notify.error(String(e));
                                    }
                                }}
                            >
                                Экспорт в Markdown
                            </Menu.Item>
                        </Menu.Dropdown>
                    </Menu>
                )}
                <Tooltip label={rightSidebarOpen ? "Скрыть правую панель" : "Показать правую панель"}>
                    <ActionIcon
                        variant="subtle"
                        size="lg"
                        onClick={onToggleRightSidebar}
                        aria-label={rightSidebarOpen ? "Скрыть правую панель" : "Показать правую панель"}
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
                title="Кастомный системный промпт"
                size="md"
                opened={customModalOpen}
                onClose={() => setCustomModalOpen(false)}
            >
                <Stack gap="sm">
                    <Textarea
                        placeholder="Введите системный промпт..."
                        value={customPromptDraft}
                        onChange={(e) => setCustomPromptDraft(e.currentTarget.value)}
                        minRows={3}
                        maxRows={10}
                        autosize
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={() => setCustomModalOpen(false)}>
                            Отмена
                        </Button>
                        <Button onClick={saveCustomPrompt}>Сохранить</Button>
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
                            Начните общение
                        </Title>
                        <Text size="sm" c="dimmed" ta="center">
                            Создайте новый чат или выберите существующий из списка
                        </Text>
                        <Button variant="light" leftSection={<IconPlus size={16} />} onClick={onNewChat}>
                            Новый чат
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
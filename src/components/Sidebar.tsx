import { useState } from "react";
import {
    ActionIcon,
    Box,
    Button,
    Group,
    NavLink,
    Popover,
    ScrollArea,
    Text,
    Tooltip,
    useMantineColorScheme,
} from "@mantine/core";
import { IconMessages, IconMoon, IconPlus, IconSettings, IconSun, IconTemplate } from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { formatRelativeDate } from "../utils/formatDate";
import { ConfirmModal } from "./ConfirmModal";

interface SidebarProps {
    width?: number;
    style?: React.CSSProperties;
    compact?: boolean;
    onNewChat: () => void;
}

export function Sidebar({ width = 260, style, compact = false, onNewChat }: SidebarProps) {
    const { colorScheme, toggleColorScheme } = useMantineColorScheme();
    const { chats, activeChatId, deleteChat, setActiveChat, setView } =
        useChatStore();
    const [deletingChatId, setDeletingChatId] = useState<string | null>(null);
    const [chatsPopoverOpened, setChatsPopoverOpened] = useState(false);

    return (
        <Box
            style={{
                width: compact ? 60 : width,
                flexShrink: 0,
                minWidth: compact ? 60 : width,
                height: "100vh",
                display: "flex",
                flexDirection: "column",
                borderRight: "1px solid var(--mantine-color-default-border)",
                ...style,
            }}
        >
            <Box p={compact ? "xs" : "md"} style={compact ? { display: "flex", justifyContent: "center" } : undefined}>
                {compact ? (
                    <Tooltip label="Новый чат">
                        <ActionIcon
                            size="lg"
                            radius="xl"
                            variant="filled"
                            onClick={onNewChat}
                            aria-label="Новый чат"
                        >
                            <IconPlus size={18} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                ) : (
                    <Button fullWidth onClick={onNewChat}>
                        + Новый чат
                    </Button>
                )}
            </Box>

            {compact ? (
                <Box style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }} p="xs">
                    <Box style={{ display: "flex", justifyContent: "center" }}>
                        <Popover
                            position="right-start"
                            width={260}
                            opened={chatsPopoverOpened}
                            onChange={setChatsPopoverOpened}
                        >
                            <Popover.Target>
                                <Tooltip label="Чаты">
                                    <ActionIcon
                                        size="lg"
                                        radius="xl"
                                        variant="subtle"
                                        onClick={() => setChatsPopoverOpened((o) => !o)}
                                        aria-label="Чаты"
                                    >
                                        <IconMessages size={18} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            </Popover.Target>
                            <Popover.Dropdown>
                                <ScrollArea mah={400} type="scroll">
                                    {chats.map((chat) => (
                                        <Group key={chat.id} wrap="nowrap" gap={0} className="chat-item" justify="flex-start">
                                            <NavLink
                                                active={chat.id === activeChatId}
                                                label={chat.title}
                                                description={
                                                    chat.updatedAt != null ? (
                                                        <Text component="span" size="xs" c="dimmed">
                                                            {formatRelativeDate(chat.updatedAt)}
                                                        </Text>
                                                    ) : undefined
                                                }
                                                onClick={() => {
                                                    setActiveChat(chat.id);
                                                    setChatsPopoverOpened(false);
                                                }}
                                                style={{ flex: 1 }}
                                                styles={{
                                                    root: {
                                                        borderLeft: chat.id === activeChatId
                                                            ? "3px solid var(--mantine-color-blue-5)"
                                                            : "3px solid transparent",
                                                    },
                                                }}
                                            />
                                            <Tooltip label="Удалить">
                                                <ActionIcon
                                                    className="chat-delete-btn"
                                                    size="xs"
                                                    variant="subtle"
                                                    onClick={() => setDeletingChatId(chat.id)}
                                                >
                                                    ✕
                                                </ActionIcon>
                                            </Tooltip>
                                        </Group>
                                    ))}
                                </ScrollArea>
                            </Popover.Dropdown>
                        </Popover>
                    </Box>
                    <Box style={{ flex: 1 }} />
                </Box>
            ) : (
                <Box style={{ flex: 1, overflowY: "auto" }} p="xs">
                    {chats.map((chat) => (
                        <Group key={chat.id} wrap="nowrap" gap={0} className="chat-item" justify="flex-start">
                            <NavLink
                                active={chat.id === activeChatId}
                                label={chat.title}
                                description={
                                    chat.updatedAt != null ? (
                                        <Text component="span" size="xs" c="dimmed">
                                            {formatRelativeDate(chat.updatedAt)}
                                        </Text>
                                    ) : undefined
                                }
                                onClick={() => setActiveChat(chat.id)}
                                style={{ flex: 1 }}
                                styles={{
                                    root: {
                                        borderLeft: chat.id === activeChatId
                                            ? "3px solid var(--mantine-color-blue-5)"
                                            : "3px solid transparent",
                                    },
                                }}
                            />
                            <Tooltip label="Удалить">
                                <ActionIcon
                                    className="chat-delete-btn"
                                    size="xs"
                                    variant="subtle"
                                    onClick={() => setDeletingChatId(chat.id)}
                                >
                                    ✕
                                </ActionIcon>
                            </Tooltip>
                        </Group>
                    ))}
                </Box>
            )}

            <Box p="md" style={{ borderTop: "1px solid var(--mantine-color-default-border)" }}>
                <Group justify="center" gap="md">
                    <Tooltip label="Сменить тему">
                        <ActionIcon
                            size="lg"
                            variant="subtle"
                            onClick={() => toggleColorScheme()}
                            aria-label={colorScheme === "dark" ? "Светлая тема" : "Тёмная тема"}
                        >
                            {colorScheme === "dark" ? <IconSun size={18} stroke={1.5} /> : <IconMoon size={18} stroke={1.5} />}
                        </ActionIcon>
                    </Tooltip>
                    <Tooltip label="Шаблоны">
                        <ActionIcon
                            size="lg"
                            variant="subtle"
                            onClick={() => setView("snippets")}
                        >
                            <IconTemplate size={18} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                    <Tooltip label="Настройки">
                        <ActionIcon
                            size="lg"
                            variant="subtle"
                            onClick={() => setView("settings")}
                        >
                            <IconSettings size={18} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Group>
            </Box>
            <ConfirmModal
                opened={deletingChatId !== null}
                onClose={() => setDeletingChatId(null)}
                onConfirm={() => {
                    if (deletingChatId) {
                        deleteChat(deletingChatId);
                        setDeletingChatId(null);
                    }
                }}
                message="Чат и все сообщения будут удалены безвозвратно."
            />
        </Box>
    );
}
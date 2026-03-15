import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Box,
    NavLink,
    ScrollArea,
    Stack,
    Text,
    Tooltip,
} from "@mantine/core";
import {
    IconLayoutBoard,
    IconLayoutSidebarRightCollapse,
    IconLayoutSidebarRightExpand,
    IconList,
    IconPackage,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";

const TRUNCATE_LEN = 50;

interface NavigationSidebarProps {
    width?: number;
    style?: React.CSSProperties;
    compact?: boolean;
    onToggle?: () => void;
}

export function NavigationSidebar({
    width = 240,
    style,
    compact = false,
    onToggle,
}: NavigationSidebarProps) {
    const { t } = useTranslation();
    const { chats, activeChatId } = useChatStore();
    const activeMode = useChatStore((s) => s.activeMode);
    const setShowWorkspacePanel = useChatStore((s) => s.setShowWorkspacePanel);
    const activeProjectId = useChatStore((s) => s.activeProjectId);
    const openProjectDashboard = useChatStore((s) => s.openProjectDashboard);
    const [activeUserMessageId, setActiveUserMessageId] = useState<string | null>(
        null
    );

    const activeChat = chats.find((c) => c.id === activeChatId);
    const userMessages = activeChat
        ? activeChat.messages.filter((m) => m.role === "user")
        : [];

    const getAssistantIdForUserMessage = (userMessageId: string): string | null => {
        if (!activeChat) return null;
        const assistant = activeChat.messages.find(
            (m) => m.role === "assistant" && m.parentId === userMessageId
        );
        return assistant?.id ?? null;
    };

    useEffect(() => {
        if (!activeChat || userMessages.length === 0) {
            setActiveUserMessageId(null);
            return;
        }
        const assistantIds = userMessages
            .map((u) => getAssistantIdForUserMessage(u.id))
            .filter((id): id is string => id != null);

        const observer = new IntersectionObserver(
            (entries) => {
                for (const entry of entries) {
                    if (!entry.isIntersecting) continue;
                    const id = entry.target.id;
                    const userMsg = activeChat.messages.find(
                        (m) =>
                            m.role === "assistant" &&
                            m.id === id &&
                            m.parentId
                    );
                    if (userMsg?.parentId) {
                        setActiveUserMessageId(userMsg.parentId);
                        break;
                    }
                }
            },
            { root: null, rootMargin: "0px", threshold: 0.5 }
        );

        const timer = setTimeout(() => {
            for (const id of assistantIds) {
                const el = document.getElementById(id);
                if (el) observer.observe(el);
            }
        }, 100);
        return () => {
            clearTimeout(timer);
            observer.disconnect();
        };
    }, [activeChatId, activeChat?.messages.map((m) => m.id).join(",") ?? ""]);

    const handleClick = (userMessageId: string) => {
        const assistantId = getAssistantIdForUserMessage(userMessageId);
        if (assistantId) {
            document.getElementById(assistantId)?.scrollIntoView({
                behavior: "smooth",
            });
        }
    };

    if (compact) {
        return (
            <Box
                style={{
                    width: 50,
                    flexShrink: 0,
                    minWidth: 50,
                    height: "100%",
                    display: "flex",
                    flexDirection: "column",
                    borderLeft: "1px solid var(--mantine-color-default-border)",
                    ...style,
                }}
            >
                <Stack gap={4} align="center" px={4} py="xs" style={{ flex: 1 }}>
                    {userMessages.length > 0 && (
                        <Tooltip label={t("navSidebar.messages")} position="left">
                            <ActionIcon
                                variant="subtle"
                                size="lg"
                                onClick={onToggle}
                            >
                                <IconList size={20} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    )}
                    {activeMode === "assistant" && (
                        <Tooltip label={t("workspace.title")} position="left">
                            <ActionIcon
                                variant="subtle"
                                size="lg"
                                onClick={() => setShowWorkspacePanel(true)}
                            >
                                <IconPackage size={20} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    )}
                    {activeMode === "assistant" && (
                        <Tooltip
                            label={activeProjectId ? t("project.dashboard.openDashboard") : t("project.dashboard.selectProject")}
                            position="left"
                        >
                            <ActionIcon
                                variant="subtle"
                                size="lg"
                                disabled={!activeProjectId}
                                onClick={() => activeProjectId && openProjectDashboard(activeProjectId)}
                            >
                                <IconLayoutBoard size={20} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    )}
                    <Box style={{ flex: 1 }} />
                    <Tooltip label={t("navSidebar.expand")} position="left">
                        <ActionIcon variant="subtle" size="lg" onClick={onToggle}>
                            <IconLayoutSidebarRightExpand size={20} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Stack>
            </Box>
        );
    }

    return (
        <Box
            style={{
                width: width,
                flexShrink: 0,
                minWidth: width,
                height: "100%",
                display: "flex",
                flexDirection: "column",
                borderLeft: "1px solid var(--mantine-color-default-border)",
                ...style,
            }}
        >
            <Box p="xs">
                <Text size="sm" c="dimmed" fw={500}>
                    {t("navSidebar.messages")}
                </Text>
            </Box>
            {userMessages.length > 0 ? (
                <ScrollArea style={{ flex: 1 }} type="scroll">
                    <Box p="xs">
                        {userMessages.map((msg) => {
                            const label =
                                msg.content.length > TRUNCATE_LEN
                                    ? msg.content.slice(0, TRUNCATE_LEN) + "\u2026"
                                    : msg.content;
                            const tooltipLabel =
                                msg.content.length > 200
                                    ? msg.content.slice(0, 200) + "\u2026"
                                    : msg.content;
                            return (
                                <Tooltip
                                    key={msg.id}
                                    label={tooltipLabel}
                                    multiline
                                    maw={300}
                                >
                                    <NavLink
                                        active={activeUserMessageId === msg.id}
                                        label={label}
                                        onClick={() => handleClick(msg.id)}
                                        style={{ marginBottom: 4 }}
                                    />
                                </Tooltip>
                            );
                        })}
                    </Box>
                </ScrollArea>
            ) : (
                <Box style={{ flex: 1 }} />
            )}
            <Box px="sm" py="xs" style={{ borderTop: "1px solid var(--mantine-color-default-border)" }}>
                <Stack gap={4}>
                    {activeMode === "assistant" && (
                        <Tooltip label={t("workspace.title")} position="left">
                            <ActionIcon
                                variant="subtle"
                                size="lg"
                                onClick={() => setShowWorkspacePanel(true)}
                                w="100%"
                            >
                                <IconPackage size={20} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    )}
                    {activeMode === "assistant" && (
                        <Tooltip
                            label={activeProjectId ? t("project.dashboard.openDashboard") : t("project.dashboard.selectProject")}
                            position="left"
                        >
                            <ActionIcon
                                variant="subtle"
                                size="lg"
                                disabled={!activeProjectId}
                                onClick={() => activeProjectId && openProjectDashboard(activeProjectId)}
                                w="100%"
                            >
                                <IconLayoutBoard size={20} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    )}
                    <Tooltip label={t("navSidebar.collapse")} position="left">
                        <ActionIcon variant="subtle" size="lg" onClick={onToggle} w="100%">
                            <IconLayoutSidebarRightCollapse size={20} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Stack>
            </Box>
        </Box>
    );
}

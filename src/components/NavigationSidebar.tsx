import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Box, NavLink, ScrollArea, Text } from "@mantine/core";
import { useChatStore } from "../store/chatStore";

const TRUNCATE_LEN = 50;

interface NavigationSidebarProps {
    width?: number;
    style?: React.CSSProperties;
}

export function NavigationSidebar({ width = 240, style }: NavigationSidebarProps) {
    const { t } = useTranslation();
    const { chats, activeChatId } = useChatStore();
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

    if (!activeChatId || userMessages.length === 0) {
        return null;
    }

    return (
        <Box
            style={{
                width: width,
                flexShrink: 0,
                minWidth: width,
                height: "100vh",
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
            <ScrollArea style={{ flex: 1 }} type="scroll">
                <Box p="xs">
                    {userMessages.map((msg) => {
                        const label =
                            msg.content.length > TRUNCATE_LEN
                                ? msg.content.slice(0, TRUNCATE_LEN) + "…"
                                : msg.content;
                        return (
                            <NavLink
                                key={msg.id}
                                active={activeUserMessageId === msg.id}
                                label={label}
                                onClick={() => handleClick(msg.id)}
                                style={{ marginBottom: 4 }}
                            />
                        );
                    })}
                </Box>
            </ScrollArea>
        </Box>
    );
}

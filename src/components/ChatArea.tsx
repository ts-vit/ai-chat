import { useTranslation } from "react-i18next";
import { Box, Stack, Text, Title } from "@mantine/core";
import { IconMessageChatbot } from "@tabler/icons-react";
import { MessageList } from "./MessageList";
import { MessageInput } from "./MessageInput";
import { ChatStats } from "./ChatStats";
import { useChatStore } from "../store/chatStore";

interface ChatAreaProps {
    compact?: boolean;
    hideStats?: boolean;
    onNewChat: () => void;
    messageInputRef?: React.RefObject<HTMLTextAreaElement | null>;
}

const CHAT_WIDTH_STYLE: Record<string, { maxWidth?: string; margin: string }> = {
    narrow: { maxWidth: "680px", margin: "0 auto" },
    standard: { maxWidth: "900px", margin: "0 auto" },
    wide: { margin: "0" },
};

export function ChatArea({
    compact = false,
    hideStats = false,
    onNewChat: _onNewChat,
    messageInputRef,
}: ChatAreaProps) {
    const { t } = useTranslation();
    const {
        chats,
        activeChatId,
        models,
        isStreaming,
        isStopping,
        sendMessage,
        editAndResend,
        stopGeneration,
        settings,
    } = useChatStore();

    const activeChat = chats.find((c) => c.id === activeChatId);
    const chatWidth = settings.chatWidth === "narrow" || settings.chatWidth === "wide" ? settings.chatWidth : "standard";
    const chatContainerStyle = CHAT_WIDTH_STYLE[chatWidth];

    return (
        <Box
            style={{
                flex: 1,
                minWidth: 0,
                overflow: "hidden",
                display: "flex",
                flexDirection: "column",
            }}
        >
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
                    </Stack>
                </Box>
            ) : (
                <Box
                    style={{
                        flex: 1,
                        minWidth: 0,
                        overflow: "hidden",
                        display: "flex",
                        flexDirection: "column",
                        width: "100%",
                        ...chatContainerStyle,
                    }}
                >
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
                        contextLength={models.find((m) => m.id === activeChat?.model)?.context_length}
                        usedTokens={activeChat?.messages.reduce(
                            (sum, m) =>
                                sum +
                                (m.promptTokens ?? 0) +
                                (m.completionTokens ?? 0),
                            0
                        )}
                    />
                </Box>
            )}
        </Box>
    );
}

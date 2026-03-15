import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Box, Button, Text, SimpleGrid, Paper } from "@mantine/core";
import { IconPlus, IconRobot } from "@tabler/icons-react";
import { MessageList } from "./MessageList";
import { MessageInput } from "./MessageInput";
import { ImageConfigBar } from "./ImageConfigBar";
import { ChatStats } from "./ChatStats";
import { AgentStatusBar } from "./AgentStatusBar";
import { VariablesModal, getUniqueVariableNames } from "./VariablesModal";
import { useChatStore } from "../store/chatStore";
import logo from "../assets/sp-logo.png";

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
    onNewChat,
    messageInputRef,
}: ChatAreaProps) {
    const {
        chats,
        activeChatId,
        models,
        isStreaming,
        isStopping,
        sendMessage,
        editAndResend,
        switchBranch,
        stopGeneration,
        settings,
        welcomeSnippets,
        setInsertSnippetText,
    } = useChatStore();

    const activeMode = useChatStore((s) => s.activeMode);
    const agentStatus = useChatStore((s) => s.agentStatus);
    const agentIteration = useChatStore((s) => s.agentIteration);
    const agentMaxIterations = useChatStore((s) => s.agentMaxIterations);
    const currentAgentRun = useChatStore((s) => s.currentAgentRun);
    const activeSubAgents = useChatStore((s) => s.activeSubAgents);
    const sendAgentMessage = useChatStore((s) => s.sendAgentMessage);
    const cancelAgentRun = useChatStore((s) => s.cancelAgentRun);
    const cancelSubAgentRun = useChatStore((s) => s.cancelSubAgentRun);
    const resumeAgentRun = useChatStore((s) => s.resumeAgentRun);

    const handleSend = activeMode === "assistant" ? sendAgentMessage : sendMessage;

    const { t } = useTranslation();
    const [variablesModalOpen, setVariablesModalOpen] = useState(false);
    const [variablesModalContent, setVariablesModalContent] = useState("");

    const activeChat = chats.find((c) => c.id === activeChatId);
    const chatWidth = settings.chatWidth === "narrow" || settings.chatWidth === "wide" ? settings.chatWidth : "standard";
    const chatContainerStyle = CHAT_WIDTH_STYLE[chatWidth];
    const hasMessages = activeChat && activeChat.messages.length > 0;

    const handleSuggestClick = (content: string) => {
        const vars = getUniqueVariableNames(content);
        if (vars.length === 0) {
            setInsertSnippetText(content);
            return;
        }
        setVariablesModalContent(content);
        setVariablesModalOpen(true);
    };

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
                        flexDirection: "column",
                        justifyContent: "center",
                        alignItems: "center",
                        height: "100%",
                    }}
                >
                    <img
                        src={logo}
                        alt=""
                        style={{ height: 96, display: "block", marginBottom: 12 }}
                    />
                    <Text
                        fw={700}
                        style={{
                            fontFamily: "'JetBrains Mono', monospace",
                            fontSize: 26,
                            letterSpacing: 2,
                            color: "#D4854A",
                            marginBottom: 8,
                        }}
                    >
                        UNI AI
                    </Text>
                    <Text size="sm" c="dimmed" mb={20}>
                        {t("startScreen.subtitle")}
                    </Text>
                    <Button
                        variant="light"
                        color="brand"
                        size="md"
                        leftSection={<IconPlus size={18} stroke={1.5} />}
                        onClick={onNewChat}
                    >
                        {t("startScreen.newChat")}
                    </Button>
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
                    {hasMessages && (
                        <>
                            <MessageList
                                messages={activeChat.messages}
                                isStreaming={isStreaming}
                                onEditResend={editAndResend}
                                onSwitchBranch={switchBranch}
                                compact={compact}
                            />
                            {!hideStats && (
                                <ChatStats
                                    compact={compact}
                                    providerId={activeChat?.providerId ?? "openrouter"}
                                />
                            )}
                        </>
                    )}
                    <Box className={`message-input-positioner${!hasMessages ? " centered" : ""}`}>
                        {!hasMessages && (
                            <Box className="welcome-screen">
                                {activeMode === "assistant" ? (
                                    <>
                                        <IconRobot size={88} stroke={1} style={{ marginBottom: 14, color: "var(--mantine-color-teal-5)" }} />
                                        <Text size="xl" fw={600} ta="center" mb={4}>
                                            {t("modes.assistantWelcome")}
                                        </Text>
                                        <Text size="sm" c="dimmed" ta="center" mb="lg">
                                            {t("modes.assistantWelcomeSubtitle")}
                                        </Text>
                                    </>
                                ) : (
                                    <>
                                        <img
                                            src={logo}
                                            alt=""
                                            style={{ height: 88, display: "block", marginBottom: 14 }}
                                        />
                                        <Text size="xl" fw={600} ta="center" mb={4}>
                                            {t("welcome.title")}
                                        </Text>
                                        <Text size="sm" c="dimmed" ta="center" mb="lg">
                                            {t("welcome.subtitle")}
                                        </Text>
                                        {welcomeSnippets.length > 0 && (
                                            <SimpleGrid cols={{ base: 1, xs: 2 }} spacing="sm" style={{ width: "100%" }}>
                                                {welcomeSnippets.map((s) => (
                                                    <Paper
                                                        key={s.id}
                                                        p="sm"
                                                        radius="md"
                                                        className="welcome-suggest-card"
                                                        onClick={() => handleSuggestClick(s.content)}
                                                        style={{ cursor: "pointer" }}
                                                    >
                                                        <Text size="sm" fw={500} lineClamp={1}>{s.name}</Text>
                                                        <Text size="xs" c="dimmed" lineClamp={2}>{s.content}</Text>
                                                    </Paper>
                                                ))}
                                            </SimpleGrid>
                                        )}
                                    </>
                                )}
                            </Box>
                        )}
                        {activeChat && models.find((m) => m.id === activeChat.model)?.supportsImageGeneration && (
                            <ImageConfigBar chatId={activeChat.id} chat={activeChat} />
                        )}
                        {(agentStatus === 'running' || agentStatus === 'paused') && (
                            <AgentStatusBar
                                iteration={agentIteration}
                                maxIterations={agentMaxIterations}
                                status={agentStatus}
                                onCancel={cancelAgentRun}
                                onResume={resumeAgentRun}
                                assignedModel={currentAgentRun?.assignedModel}
                                runCost={currentAgentRun?.cost}
                                chatDefaultModel={activeChat?.model}
                                subAgents={Object.values(activeSubAgents)}
                                onCancelSubAgent={cancelSubAgentRun}
                            />
                        )}
                        <MessageInput
                            onSend={handleSend}
                            onStop={activeMode === "assistant" ? cancelAgentRun : stopGeneration}
                            disabled={isStreaming}
                            isStopping={isStopping}
                            compact={compact}
                            centered={!hasMessages}
                            inputRef={messageInputRef}
                            contextLength={models.find((m) => m.id === activeChat?.model)?.context_length}
                            usedTokens={activeChat?.messages.reduce(
                                (sum, m) =>
                                    sum +
                                    (m.promptTokens ?? 0) +
                                    (m.completionTokens ?? 0),
                                0
                            )}
                            activeChat={activeChat}
                        />
                    </Box>
                </Box>
            )}
            <VariablesModal
                content={variablesModalContent}
                opened={variablesModalOpen}
                onClose={() => setVariablesModalOpen(false)}
                onSubmit={(filledText) => {
                    setInsertSnippetText(filledText);
                    setVariablesModalOpen(false);
                }}
            />
        </Box>
    );
}

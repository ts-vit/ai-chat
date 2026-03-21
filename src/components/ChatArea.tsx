import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Box, Button, Text, SimpleGrid, Paper, Stack } from "@mantine/core";
import { IconPlus, IconRobot, IconWand, IconSubtask, IconWorldSearch, IconFileImport } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { AssistantDashboard } from "./AssistantDashboard";
import { MessageList } from "./MessageList";
import { MessageInput } from "./MessageInput";
import { ImageConfigBar } from "./ImageConfigBar";
import { ChatStats } from "./ChatStats";
import { AgentStatusBar } from "./AgentStatusBar";
import AgentTracePanel from "./AgentTracePanel";
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
    const contextUsage = useChatStore((s) => s.contextUsage);
    const sendAgentMessage = useChatStore((s) => s.sendAgentMessage);
    const cancelAgentRun = useChatStore((s) => s.cancelAgentRun);
    const cancelSubAgentRun = useChatStore((s) => s.cancelSubAgentRun);
    const resumeAgentRun = useChatStore((s) => s.resumeAgentRun);
    const agentTrace = useChatStore((s) => s.agentTrace);
    const showTracePanel = useChatStore((s) => s.showTracePanel);
    const setShowTracePanel = useChatStore((s) => s.setShowTracePanel);

    const createKnowledgeBase = useChatStore((s) => s.createKnowledgeBase);
    const addKbDocuments = useChatStore((s) => s.addKbDocuments);
    const indexAllKbDocuments = useChatStore((s) => s.indexAllKbDocuments);
    const attachKbToChat = useChatStore((s) => s.attachKbToChat);
    const handleSend = activeMode === "assistant" ? sendAgentMessage : sendMessage;

    const { t } = useTranslation();
    const [variablesModalOpen, setVariablesModalOpen] = useState(false);
    const [variablesModalContent, setVariablesModalContent] = useState("");
    const [isDragOver, setIsDragOver] = useState(false);
    const [isModeTransitioning, setIsModeTransitioning] = useState(false);
    const prevModeRef = useRef(activeMode);

    useEffect(() => {
        if (prevModeRef.current !== activeMode) {
            setIsModeTransitioning(true);
            const timer = setTimeout(() => setIsModeTransitioning(false), 200);
            prevModeRef.current = activeMode;
            return () => clearTimeout(timer);
        }
    }, [activeMode]);

    const activeChat = chats.find((c) => c.id === activeChatId);

    // Drag & drop documents for assistant mode active chats
    useEffect(() => {
        if (activeMode !== "assistant" || !activeChat) return;

        const supportedExts = new Set([
            ".txt", ".md", ".html", ".htm", ".pdf", ".docx", ".json", ".csv", ".xml",
            ".rst", ".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".java",
            ".c", ".cpp", ".h", ".rb", ".php", ".swift", ".kt", ".cs", ".yaml", ".yml", ".toml",
        ]);

        const handleDrop = async (event: { payload: { paths: string[] } }) => {
            const paths = event.payload.paths;
            setIsDragOver(false);
            if (!paths || paths.length === 0) return;

            const validPaths = paths.filter((p) => {
                const ext = p.substring(p.lastIndexOf(".")).toLowerCase();
                return supportedExts.has(ext);
            });
            if (validPaths.length === 0) return;

            try {
                const currentKbId = useChatStore.getState().chatKbId;
                if (currentKbId) {
                    await addKbDocuments(currentKbId, validPaths);
                    await indexAllKbDocuments(currentKbId);
                } else {
                    const name = `Documents ${new Date().toLocaleDateString()}`;
                    const kb = await createKnowledgeBase(name, "Auto-created from chat drop");
                    if (!kb) return;
                    await addKbDocuments(kb.id, validPaths);
                    await indexAllKbDocuments(kb.id);
                    await attachKbToChat(activeChat.id, kb.id);
                }
            } catch (e) {
                console.error("Drop error:", e);
            }
        };

        const unlistenDrop = listen<{ paths: string[] }>("tauri://drag-drop", handleDrop as never);
        const unlistenOver = listen("tauri://drag-over", () => setIsDragOver(true));
        const unlistenLeave = listen("tauri://drag-leave", () => setIsDragOver(false));

        return () => {
            unlistenDrop.then((f) => f());
            unlistenOver.then((f) => f());
            unlistenLeave.then((f) => f());
        };
    }, [activeMode, activeChat?.id]);

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
                        opacity: isModeTransitioning ? 0 : 1,
                        transform: isModeTransitioning ? "translateY(4px)" : "translateY(0)",
                        transition: "opacity 200ms ease, transform 200ms ease",
                    }}
                >
                {activeMode === "assistant" ? (
                    <AssistantDashboard onNewChat={onNewChat} />
                ) : (
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
                            onClick={() => onNewChat()}
                        >
                            {t("startScreen.newChat")}
                        </Button>
                    </Box>
                )}
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
                        position: "relative",
                        ...chatContainerStyle,
                    }}
                >
                    {/* Drag overlay for assistant mode */}
                    {isDragOver && activeMode === "assistant" && (
                        <Box
                            pos="absolute"
                            top={0} left={0} right={0} bottom={0}
                            style={{
                                background: "rgba(var(--mantine-color-teal-5-rgb, 32, 178, 170), 0.08)",
                                border: "2px dashed var(--mantine-color-teal-5)",
                                borderRadius: 8,
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                zIndex: 100,
                            }}
                        >
                            <Stack align="center" gap="xs">
                                <IconFileImport size={48} stroke={1.5} color="var(--mantine-color-teal-5)" />
                                <Text size="lg" fw={600} c="teal">
                                    {t("assistantDashboard.dropDocuments")}
                                </Text>
                            </Stack>
                        </Box>
                    )}
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
                                        <Text size="sm" c="dimmed" ta="center" mb="md">
                                            {t("modes.assistantWelcomeSubtitle")}
                                        </Text>
                                        <SimpleGrid cols={{ base: 1, xs: 3 }} spacing="xs" style={{ width: "100%" }}>
                                            <Button variant="light" color="teal" size="xs" leftSection={<IconWand size={14} stroke={1.5} />} onClick={() => handleSuggestClick("Analyze this in detail: ")}>
                                                {t("assistantDashboard.chipAnalyze")}
                                            </Button>
                                            <Button variant="light" color="teal" size="xs" leftSection={<IconSubtask size={14} stroke={1.5} />} onClick={() => handleSuggestClick("/plan ")}>
                                                {t("assistantDashboard.chipPlan")}
                                            </Button>
                                            <Button variant="light" color="teal" size="xs" leftSection={<IconWorldSearch size={14} stroke={1.5} />} onClick={() => handleSuggestClick("Search the web for: ")}>
                                                {t("assistantDashboard.chipSearch")}
                                            </Button>
                                        </SimpleGrid>
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
                            <>
                                {showTracePanel && agentTrace.length > 0 && (
                                    <AgentTracePanel steps={agentTrace} />
                                )}
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
                                    contextUsage={contextUsage}
                                    traceStepCount={agentTrace.length}
                                    showTracePanel={showTracePanel}
                                    onToggleTrace={() => setShowTracePanel(!showTracePanel)}
                                />
                            </>
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

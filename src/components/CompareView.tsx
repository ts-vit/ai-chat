import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Badge,
    Box,
    Button,
    Group,
    Loader,
    Modal,
    Paper,
    Stack,
    Text,
    TextInput,
    Tooltip,
} from "@mantine/core";
import {
    IconArrowLeft,
    IconColumns,
    IconPlus,
} from "@tabler/icons-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir } from "@tauri-apps/api/path";
import { useChatStore } from "../store/chatStore";
import { CreateComparisonModal } from "./CreateComparisonModal";
import { MessageInput } from "./MessageInput";
import type { Comparison, ComparisonMessage, ContentBlock } from "../types";
import { ToolCallBlock } from "./ToolCallBlock";

function formatTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return n.toLocaleString();
}

interface CompareStatsProps {
    messages: ComparisonMessage[];
    side: "left" | "right";
}

function CompareStats({ messages, side }: CompareStatsProps) {
    const { t } = useTranslation();
    const sideMessages = messages.filter((m) => m.role === "assistant" && m.side === side);
    const totalTokens = sideMessages.reduce(
        (sum, m) => sum + (m.promptTokens ?? 0) + (m.completionTokens ?? 0),
        0
    );
    const totalCost = sideMessages.reduce((sum, m) => sum + (m.cost ?? 0), 0);

    return (
        <Box
            px="xs"
            py={4}
            style={{
                backgroundColor: "var(--mantine-color-default-hover)",
                borderTop: "1px solid var(--mantine-color-default-border)",
            }}
        >
            <Group gap="xs" wrap="nowrap" justify="flex-start">
                <Tooltip label={t("chatStats.tokensInChat")}>
                    <Text component="span" size="xs" style={{ whiteSpace: "nowrap" }}>
                        {formatTokens(totalTokens)} tok
                    </Text>
                </Tooltip>
                <Text component="span" size="xs" c="dark.3" style={{ whiteSpace: "nowrap" }}>|</Text>
                <Tooltip label={t("chatStats.costOfChat")}>
                    <Text component="span" size="xs" style={{ whiteSpace: "nowrap" }}>
                        ${totalCost.toFixed(4)}
                    </Text>
                </Tooltip>
            </Group>
        </Box>
    );
}

interface Round {
    userMessage: ComparisonMessage;
    leftAssistant?: ComparisonMessage;
    rightAssistant?: ComparisonMessage;
}

function buildRounds(messages: ComparisonMessage[]): Round[] {
    const rounds: Round[] = [];
    let current: Round | null = null;

    for (const msg of messages) {
        if (msg.role === "user") {
            if (current) rounds.push(current);
            current = { userMessage: msg };
        } else if (msg.role === "assistant" && current) {
            if (msg.side === "left") current.leftAssistant = msg;
            if (msg.side === "right") current.rightAssistant = msg;
        }
    }
    if (current) rounds.push(current);
    return rounds;
}

function tryParseContentBlocks(content: string): ContentBlock[] | null {
    const trimmed = content.trimStart();
    if (!trimmed.startsWith("[")) return null;
    try {
        const parsed = JSON.parse(content) as unknown;
        if (!Array.isArray(parsed)) return null;
        const valid = parsed.every(
            (b: unknown) =>
                typeof b === "object" && b !== null && typeof (b as ContentBlock).type === "string"
        );
        return valid ? (parsed as ContentBlock[]) : null;
    } catch {
        return null;
    }
}

function AssistantBubble({
    content,
    isStreaming,
    appDataDirPath,
    onImageClick,
}: {
    content: string;
    isStreaming: boolean;
    appDataDirPath: string | null;
    onImageClick?: (src: string) => void;
}) {
    const blocks = content ? tryParseContentBlocks(content) : null;
    const sep = appDataDirPath?.includes("\\") ? "\\" : "/";
    const base = appDataDirPath?.replace(/[/\\]+$/, "") ?? "";

    if (blocks) {
        return (
            <Box style={{ flex: 1, minWidth: 0, overflow: "hidden" }}>
                {blocks.map((block) => {
                    if (block.type === "image") {
                        if (!appDataDirPath) return null;
                        const path = "path" in block ? block.path : "";
                        const isAbsolute =
                            path.startsWith("/") || /^[A-Za-z]:[/\\]/.test(path);
                        const fullPath = isAbsolute
                            ? path
                            : base + sep + path.replace(/^[/\\]+/, "").replace(/\//g, sep);
                        const src = convertFileSrc(fullPath);
                        return (
                            <Box key={`image-${path}`} mb="xs">
                                <img
                                    src={src}
                                    alt={"name" in block ? block.name : "image"}
                                    style={{
                                        maxWidth: 400,
                                        borderRadius: "var(--mantine-radius-sm)",
                                        cursor: "pointer",
                                        display: "block",
                                    }}
                                    role="button"
                                    tabIndex={0}
                                    onClick={() => onImageClick?.(src)}
                                    onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onImageClick?.(src); } }}
                                />
                            </Box>
                        );
                    }
                    if (block.type === "text" && "text" in block && block.text) {
                        return (
                            <Box key={`text-${block.text.slice(0, 32)}`} className="markdown-body" style={{ fontSize: 14 }}>
                                <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
                                    {block.text}
                                </ReactMarkdown>
                            </Box>
                        );
                    }
                    return null;
                })}
            </Box>
        );
    }

    return (
        <Box style={{ flex: 1, minWidth: 0, overflow: "hidden" }}>
            {content ? (
                <Box className="markdown-body" style={{ fontSize: 14 }}>
                    <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
                        {content}
                    </ReactMarkdown>
                </Box>
            ) : isStreaming ? (
                <Group gap="xs">
                    <Loader size="xs" />
                    <Text size="xs" c="dimmed">...</Text>
                </Group>
            ) : (
                <Text size="xs" c="dimmed">—</Text>
            )}
        </Box>
    );
}

function UserMessageContent({
    message,
    appDataDirPath,
    onImageClick,
}: {
    message: ComparisonMessage;
    appDataDirPath: string | null;
    onImageClick?: (src: string) => void;
}) {
    const blocks = message.hasAttachments ? tryParseContentBlocks(message.content) : null;
    if (!blocks) {
        return (
            <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
                {message.content}
            </Text>
        );
    }
    const sep = appDataDirPath?.includes("\\") ? "\\" : "/";
    const base = appDataDirPath?.replace(/[/\\]+$/, "") ?? "";
    return (
        <Box>
            {blocks.map((block) => {
                if (block.type === "image" && "path" in block && block.path && appDataDirPath) {
                    const isAbs = block.path.startsWith("/") || /^[A-Za-z]:[/\\]/.test(block.path);
                    const full = isAbs ? block.path : base + sep + block.path.replace(/^[/\\]+/, "").replace(/\//g, sep);
                    const src = convertFileSrc(full);
                    return (
                        <Box key={`image-${block.path}`} mb="xs">
                            <img
                                src={src}
                                alt={"name" in block ? block.name : "image"}
                                style={{ maxWidth: 200, borderRadius: "var(--mantine-radius-sm)", cursor: "pointer", display: "block" }}
                                role="button"
                                tabIndex={0}
                                onClick={() => onImageClick?.(src)}
                                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onImageClick?.(src); } }}
                            />
                        </Box>
                    );
                }
                if (block.type === "file" && "name" in block) {
                    return (
                        <Badge key={`file-${block.name}`} size="xs" variant="light" mb="xs">
                            {block.name}
                        </Badge>
                    );
                }
                if (block.type === "text" && "text" in block && block.text) {
                    return (
                        <Text key={`text-${block.text.slice(0, 32)}`} size="sm" style={{ whiteSpace: "pre-wrap" }}>
                            {block.text}
                        </Text>
                    );
                }
                return null;
            })}
        </Box>
    );
}

export function CompareView() {
    const { t } = useTranslation();
    const {
        comparisons,
        activeComparisonId,
        comparisonMessages,
        compareStreamingLeft,
        compareStreamingRight,
        compareToolCallsLeft,
        compareToolCallsRight,
        loadComparisons,
        setActiveComparison,
        setView,
        createComparison,
        updateComparisonTitle,
        sendComparisonMessage,
        stopComparisonGeneration,
    } = useChatStore();

    const [createModalOpen, setCreateModalOpen] = useState(false);
    const [editingTitle, setEditingTitle] = useState<string | null>(null);
    const [titleDraft, setTitleDraft] = useState("");
    const [appDataDirPath, setAppDataDirPath] = useState<string | null>(null);
    const [lightboxSrc, setLightboxSrc] = useState<string | null>(null);
    const bottomRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        appDataDir().then(setAppDataDirPath).catch(() => setAppDataDirPath(null));
    }, []);

    useEffect(() => {
        loadComparisons();
    }, [loadComparisons]);

    useEffect(() => {
        bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [comparisonMessages]);

    useEffect(() => {
        if (!activeComparisonId && comparisons.length > 0) {
            setActiveComparison(comparisons[0].id);
        }
    }, [activeComparisonId, comparisons, setActiveComparison]);

    const activeComparison = comparisons.find((c) => c.id === activeComparisonId);
    const rounds = useMemo(() => buildRounds(comparisonMessages), [comparisonMessages]);
    const isStreaming = compareStreamingLeft || compareStreamingRight;

    const handleSend = useCallback(
        (content: string) => {
            sendComparisonMessage(content);
        },
        [sendComparisonMessage]
    );

    const handleStop = useCallback(() => {
        stopComparisonGeneration();
    }, [stopComparisonGeneration]);

    const startEditTitle = useCallback((comp: Comparison) => {
        setEditingTitle(comp.id);
        setTitleDraft(comp.title);
    }, []);

    const saveTitle = useCallback(() => {
        if (editingTitle && titleDraft.trim()) {
            updateComparisonTitle(editingTitle, titleDraft.trim());
        }
        setEditingTitle(null);
    }, [editingTitle, titleDraft, updateComparisonTitle]);

    if (!activeComparison) {
        return (
            <Box style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>
                <Box style={{ flex: 1, overflow: "auto", display: "flex", alignItems: "center", justifyContent: "center" }} p="md">
                    <Stack align="center" gap="md">
                        <Box c="dimmed">
                            <IconColumns size={64} stroke={1.2} />
                        </Box>
                        <Text size="sm" c="dimmed" ta="center">
                            {t("compare.emptyState")}
                        </Text>
                        <Button
                            size="xs"
                            leftSection={<IconPlus size={14} stroke={1.5} />}
                            onClick={() => setCreateModalOpen(true)}
                        >
                            {t("compare.newComparison")}
                        </Button>
                    </Stack>
                </Box>
                <CreateComparisonModal
                    opened={createModalOpen}
                    onClose={() => setCreateModalOpen(false)}
                    onConfirm={(a, b, c, d, e, f) => {
                        createComparison(a, b, c, d, e, f);
                        setView("compare");
                    }}
                />
            </Box>
        );
    }

    return (
        <Box style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>
            {/* Header */}
            <Box
                px="md"
                py="xs"
                style={{
                    borderBottom: "1px solid var(--mantine-color-default-border)",
                    flexShrink: 0,
                }}
            >
                <Group justify="space-between" wrap="nowrap">
                    <Group gap="xs" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
                        <Group gap={4} wrap="nowrap" style={{ cursor: "pointer", flexShrink: 0 }} onClick={() => setView("comparisons")}>
                            <IconArrowLeft size={16} stroke={1.5} color="var(--mantine-color-dimmed)" />
                            <Text size="sm" c="dimmed">{t("comparisons.back")}</Text>
                        </Group>
                        {editingTitle === activeComparison.id ? (
                            <TextInput
                                size="xs"
                                value={titleDraft}
                                onChange={(e) => setTitleDraft(e.currentTarget.value)}
                                onKeyDown={(e) => {
                                    if (e.key === "Enter") saveTitle();
                                    if (e.key === "Escape") setEditingTitle(null);
                                }}
                                onBlur={saveTitle}
                                autoFocus
                                style={{ flex: 1, minWidth: 0 }}
                            />
                        ) : (
                            <Text
                                size="sm"
                                fw={600}
                                lineClamp={1}
                                style={{ cursor: "pointer", flex: 1, minWidth: 0 }}
                                onClick={() => startEditTitle(activeComparison)}
                            >
                                {activeComparison.title}
                            </Text>
                        )}
                    </Group>
                    <Group gap="xs" wrap="nowrap">
                        <Badge size="xs" variant="light" color="brand">
                            {activeComparison.leftModel.split("/").pop()}
                        </Badge>
                        <Text size="xs" c="dimmed">{t("compare.vsLabel")}</Text>
                        <Badge size="xs" variant="light" color="grape">
                            {activeComparison.rightModel.split("/").pop()}
                        </Badge>
                    </Group>
                </Group>
            </Box>

            {/* Split view */}
            <Box style={{ flex: 1, overflow: "auto" }}>
                {rounds.length === 0 ? (
                    <Stack align="center" gap="md" mt="xl">
                        <Text size="sm" c="dimmed" ta="center">
                            {t("compare.emptyActive")}
                        </Text>
                    </Stack>
                ) : (
                    <Box p="md">
                        {rounds.map((round, i) => (
                            <Box key={round.userMessage.id} mb="lg">
                                {/* User message */}
                                <Paper
                                    p="sm"
                                    mb="sm"
                                    radius="sm"
                                    style={{
                                        backgroundColor: "var(--mantine-color-brand-light)",
                                    }}
                                >
                                    <Text size="xs" c="dimmed" mb={4}>
                                        {t("compare.round", { n: i + 1 })}
                                    </Text>
                                    <UserMessageContent
                                        message={round.userMessage}
                                        appDataDirPath={appDataDirPath}
                                        onImageClick={setLightboxSrc}
                                    />
                                </Paper>

                                {/* Two columns */}
                                <Box style={{ display: "flex", gap: 12 }}>
                                    {/* Left */}
                                    <Paper
                                        p="sm"
                                        radius="sm"
                                        withBorder
                                        style={{
                                            flex: 1,
                                            minWidth: 0,
                                            borderColor: "var(--mantine-color-brand-3)",
                                            borderWidth: 1,
                                        }}
                                    >
                                        <AssistantBubble
                                            content={round.leftAssistant?.content ?? ""}
                                            isStreaming={compareStreamingLeft && i === rounds.length - 1}
                                            appDataDirPath={appDataDirPath}
                                            onImageClick={setLightboxSrc}
                                        />
                                        {compareStreamingLeft && i === rounds.length - 1 && compareToolCallsLeft.length > 0 && (
                                            <ToolCallBlock toolCalls={compareToolCallsLeft} />
                                        )}
                                    </Paper>

                                    {/* Right */}
                                    <Paper
                                        p="sm"
                                        radius="sm"
                                        withBorder
                                        style={{
                                            flex: 1,
                                            minWidth: 0,
                                            borderColor: "var(--mantine-color-grape-3)",
                                            borderWidth: 1,
                                        }}
                                    >
                                        <AssistantBubble
                                            content={round.rightAssistant?.content ?? ""}
                                            isStreaming={compareStreamingRight && i === rounds.length - 1}
                                            appDataDirPath={appDataDirPath}
                                            onImageClick={setLightboxSrc}
                                        />
                                        {compareStreamingRight && i === rounds.length - 1 && compareToolCallsRight.length > 0 && (
                                            <ToolCallBlock toolCalls={compareToolCallsRight} />
                                        )}
                                    </Paper>
                                </Box>
                            </Box>
                        ))}
                        <div ref={bottomRef} />
                    </Box>
                )}
            </Box>

            {/* Status bars */}
            <Box style={{ display: "flex", flexShrink: 0 }}>
                <Box style={{ flex: 1 }}>
                    <CompareStats messages={comparisonMessages} side="left" />
                </Box>
                <Box style={{ flex: 1 }}>
                    <CompareStats messages={comparisonMessages} side="right" />
                </Box>
            </Box>

            {/* Input */}
            <MessageInput
                onSend={handleSend}
                onStop={handleStop}
                disabled={isStreaming}
                isStopping={false}
                enableAttachments
            />

            <CreateComparisonModal
                opened={createModalOpen}
                onClose={() => setCreateModalOpen(false)}
                onConfirm={(a, b, c, d, e, f) => {
                    createComparison(a, b, c, d, e, f);
                    setView("compare");
                }}
            />

            <Modal
                opened={lightboxSrc !== null}
                onClose={() => setLightboxSrc(null)}
                withCloseButton
                size="lg"
                padding={0}
                styles={{ body: { padding: 0 } }}
            >
                {lightboxSrc && (
                    <img
                        src={lightboxSrc}
                        alt="Full size"
                        style={{ width: "100%", display: "block" }}
                    />
                )}
            </Modal>
        </Box>
    );
}

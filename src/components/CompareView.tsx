import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
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
                {blocks.map((block, i) => {
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
                            <Box key={i} mb="xs">
                                <img
                                    src={src}
                                    alt={"name" in block ? block.name : `image_${i}`}
                                    style={{
                                        maxWidth: 400,
                                        borderRadius: "var(--mantine-radius-sm)",
                                        cursor: "pointer",
                                        display: "block",
                                    }}
                                    onClick={() => onImageClick?.(src)}
                                />
                            </Box>
                        );
                    }
                    if (block.type === "text" && "text" in block && block.text) {
                        return (
                            <Box key={i} className="markdown-body" style={{ fontSize: 14 }}>
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

export function CompareView() {
    const { t } = useTranslation();
    const {
        comparisons,
        activeComparisonId,
        comparisonMessages,
        compareStreamingLeft,
        compareStreamingRight,
        loadComparisons,
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
                        <ActionIcon
                            variant="subtle"
                            size="sm"
                            onClick={() => setView("chat")}
                            aria-label={t("common.back")}
                        >
                            ←
                        </ActionIcon>
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
                        <Badge size="xs" variant="light" color="blue">
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
                                        backgroundColor: "var(--mantine-color-blue-light)",
                                    }}
                                >
                                    <Text size="xs" c="dimmed" mb={4}>
                                        {t("compare.round", { n: i + 1 })}
                                    </Text>
                                    <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
                                        {round.userMessage.content}
                                    </Text>
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
                                            borderColor: "var(--mantine-color-blue-3)",
                                            borderWidth: 1,
                                        }}
                                    >
                                        <AssistantBubble
                                            content={round.leftAssistant?.content ?? ""}
                                            isStreaming={compareStreamingLeft && i === rounds.length - 1}
                                            appDataDirPath={appDataDirPath}
                                            onImageClick={setLightboxSrc}
                                        />
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

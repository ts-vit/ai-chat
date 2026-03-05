import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionIcon, Box, Button, Group, Modal, Paper, Text, Textarea, Tooltip } from "@mantine/core";
import { IconCheck, IconCopy, IconEdit, IconFile, IconFileText, IconPlayerStop, IconVolume } from "@tabler/icons-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { openPath } from "@tauri-apps/plugin-opener";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir, join } from "@tauri-apps/api/path";
import { useChatStore } from "../store/chatStore";
import { notify } from "../utils/notify";
import { ToolCallBlock } from "./ToolCallBlock";
import type { ContentBlock, ContentBlockText, Message } from "../types";

function tryParseContentBlocks(content: string): ContentBlock[] | null {
    const trimmed = content.trimStart();
    if (!trimmed.startsWith("[")) return null;
    try {
        const parsed = JSON.parse(content) as unknown;
        if (!Array.isArray(parsed)) return null;
        const valid = parsed.every(
            (b: unknown) =>
                typeof b === "object" &&
                b !== null &&
                typeof (b as ContentBlock).type === "string"
        );
        return valid ? (parsed as ContentBlock[]) : null;
    } catch {
        return null;
    }
}

function CodeBlock({
    children,
    ...props
}: React.ComponentPropsWithoutRef<"pre">) {
    const { t } = useTranslation();
    const wrapperRef = useRef<HTMLDivElement>(null);
    const [copied, setCopied] = useState(false);

    const handleCopy = async () => {
        const codeEl = wrapperRef.current?.querySelector("pre code");
        const text = codeEl?.textContent ?? "";
        try {
            await navigator.clipboard.writeText(text);
            setCopied(true);
            setTimeout(() => setCopied(false), 2000);
        } catch {
            // clipboard API unavailable or denied
        }
    };

    return (
        <div
            ref={wrapperRef}
            className="code-block-wrapper"
            style={{ position: "relative" }}
        >
            <Tooltip label={copied ? t("messageList.copied") : t("messageList.copyCode")}>
                <ActionIcon
                    className="copy-button"
                    size="xs"
                    variant="subtle"
                    onClick={handleCopy}
                    style={{ position: "absolute", top: 8, right: 8 }}
                    aria-label={copied ? t("messageList.copied") : t("messageList.copyCode")}
                >
                    {copied ? (
                        <IconCheck size={14} stroke={1.5} color="var(--mantine-color-green-6)" />
                    ) : (
                        <IconCopy size={14} stroke={1.5} />
                    )}
                </ActionIcon>
            </Tooltip>
            <pre {...props}>{children}</pre>
        </div>
    );
}

interface Props {
    messages: Message[];
    isStreaming: boolean;
    onEditResend: (messageId: string, newContent: string) => Promise<void>;
    compact?: boolean;
}

const DENSITY_PADDING = { compact: "xs", standard: "sm", spacious: "md" } as const;
const DENSITY_MB = { compact: "xs", standard: "sm", spacious: "lg" } as const;

export function MessageList({ messages, isStreaming, onEditResend, compact = false }: Props) {
    const { t } = useTranslation();
    const { settings, scrollTargetId, setScrollTargetId, activeToolCalls, playingMessageId, speakMessage, stopTts } = useChatStore();
    const density = (settings.messageDensity === "compact" || settings.messageDensity === "spacious"
        ? settings.messageDensity
        : "standard") as keyof typeof DENSITY_PADDING;
    const listPadding = DENSITY_PADDING[density];
    const messageMb = DENSITY_MB[density];
    const bottomRef = useRef<HTMLDivElement>(null);
    const [copiedMessageId, setCopiedMessageId] = useState<string | null>(null);
    const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
    const [editContent, setEditContent] = useState("");
    const [appDataDirPath, setAppDataDirPath] = useState<string | null>(null);
    const [lightboxSrc, setLightboxSrc] = useState<string | null>(null);

    useEffect(() => {
        appDataDir().then(setAppDataDirPath).catch(() => setAppDataDirPath(null));
    }, []);

    // Автоскролл при новых сообщениях и стриминге
    useEffect(() => {
        bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [messages, isStreaming]);

    // Скролл к сообщению из поиска (scrollTargetId)
    useEffect(() => {
        if (!scrollTargetId) return;
        const el = document.getElementById(scrollTargetId);
        if (!el) return;
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        const node = el as HTMLElement;
        const prevOutline = node.style.outline;
        node.style.outline = "2px solid var(--mantine-color-blue-5)";
        const t = setTimeout(() => {
            node.style.outline = prevOutline;
            setScrollTargetId(null);
        }, 2000);
        return () => clearTimeout(t);
    }, [scrollTargetId, setScrollTargetId]);

    const handleOpenFile = useCallback(async (relativePath: string) => {
        if (!appDataDirPath) return;
        try {
            const fullPath = await join(appDataDirPath, relativePath);
            await openPath(fullPath);
        } catch (e) {
            notify.error(String(e));
        }
    }, [appDataDirPath]);

    const handleCopyMessage = async (content: string, messageId: string) => {
        try {
            await navigator.clipboard.writeText(content);
            setCopiedMessageId(messageId);
            setTimeout(() => setCopiedMessageId(null), 2000);
        } catch {
            // clipboard API unavailable or denied
        }
    };

    const renderUserMessageBody = useCallback(
        (content: string) => {
            const blocks = tryParseContentBlocks(content);
            if (!blocks || blocks.length === 0) {
                return (
                    <Text
                        size="sm"
                        style={{
                            whiteSpace: "pre-wrap",
                            fontSize: settings.font_size ?? 14,
                        }}
                    >
                        {content}
                    </Text>
                );
            }
            const sep = appDataDirPath?.includes("\\") ? "\\" : "/";
            const base = appDataDirPath?.replace(/[/\\]+$/, "") ?? "";
            return (
                <Box>
                    {blocks.map((block, i) => {
                        if (block.type === "image") {
                            if (!appDataDirPath) return <Text key={i} size="xs" c="dimmed">{t("common.loading")}</Text>;
                            const isAbsolute =
                                block.path.startsWith("/") || /^[A-Za-z]:[/\\]/.test(block.path);
                            const fullPath = isAbsolute
                                ? block.path
                                : base + sep + block.path.replace(/^[/\\]+/, "").replace(/\//g, sep);
                            const src = convertFileSrc(fullPath);
                            console.log("[image block]", { path: block.path, fullPath, src });
                            return (
                                <Box key={i} mb="xs">
                                    <img
                                        src={src}
                                        alt={block.name}
                                        style={{
                                            maxWidth: 400,
                                            borderRadius: "var(--mantine-radius-sm)",
                                            cursor: "pointer",
                                            display: "block",
                                        }}
                                        onClick={() => setLightboxSrc(src)}
                                    />
                                </Box>
                            );
                        }
                        if (block.type === "file") {
                            return (
                                <Paper key={i} p="xs" mb="xs" withBorder radius="sm">
                                    <Group gap="xs" wrap="nowrap">
                                        {block.mime === "application/pdf" ? (
                                            <IconFile size={20} stroke={1.5} style={{ color: "var(--mantine-color-default-color)" }} />
                                        ) : (
                                            <IconFileText size={20} stroke={1.5} style={{ color: "var(--mantine-color-default-color)" }} />
                                        )}
                                        <Text size="sm" lineClamp={1} style={{ flex: 1 }}>
                                            {block.name}
                                        </Text>
                                        {block.mime && (
                                            <Text size="xs" c="dimmed">
                                                {block.mime}
                                            </Text>
                                        )}
                                        <Button
                                            size="xs"
                                            variant="light"
                                            onClick={() => handleOpenFile(block.path)}
                                        >
                                            {t("messageList.open")}
                                        </Button>
                                    </Group>
                                </Paper>
                            );
                        }
                        if (block.type === "text") {
                            return (
                                <Box
                                    key={i}
                                    className="markdown-body"
                                    style={{ fontSize: settings.font_size ?? 14 }}
                                >
                                    <ReactMarkdown
                                        remarkPlugins={[remarkGfm]}
                                        rehypePlugins={[rehypeHighlight]}
                                        components={{
                                            pre: ({ children, ...props }) => (
                                                <CodeBlock {...props}>{children}</CodeBlock>
                                            ),
                                        }}
                                    >
                                        {block.text}
                                    </ReactMarkdown>
                                </Box>
                            );
                        }
                        return null;
                    })}
                </Box>
            );
        },
        [settings.font_size, appDataDirPath, handleOpenFile]
    );

    if (messages.length === 0) {
        return <Box style={{ flex: 1 }} />;
    }

    return (
        <Box style={{ flex: 1, overflowY: "auto" }} p={listPadding}>
            {messages.map((msg, index) => (
                <Box
                    key={msg.id}
                    id={msg.id}
                    className={isStreaming && index === messages.length - 1 ? "message-row streaming" : "message-row"}
                    mb={messageMb}
                >
                    <Box
                        style={{
                            display: "flex",
                            justifyContent: msg.role === "user" ? "flex-end" : "flex-start",
                        }}
                    >
                        {msg.role === "user" && editingMessageId === msg.id ? (
                            <Box style={{ maxWidth: "70%", width: "100%" }}>
                                <Textarea
                                    value={editContent}
                                    onChange={(e) => setEditContent(e.currentTarget.value)}
                                    onKeyDown={(e) => {
                                        if (e.key === "Enter" && !e.shiftKey) {
                                            e.preventDefault();
                                            const trimmed = editContent.trim();
                                            if (trimmed && !isStreaming) {
                                                onEditResend(msg.id, trimmed).then(() =>
                                                    setEditingMessageId(null)
                                                );
                                            }
                                        }
                                    }}
                                    autosize
                                    minRows={2}
                                    maxRows={10}
                                    styles={{ input: { width: "100%" } }}
                                />
                                <Group gap="xs" mt="xs" justify="flex-end">
                                    <Button
                                        variant="subtle"
                                        size="xs"
                                        onClick={() => setEditingMessageId(null)}
                                    >
                                        {t("messageList.cancel")}
                                    </Button>
                                    <Button
                                        variant="filled"
                                        size="xs"
                                        onClick={() => {
                                            const trimmed = editContent.trim();
                                            if (trimmed && !isStreaming) {
                                                onEditResend(msg.id, trimmed).then(() =>
                                                    setEditingMessageId(null)
                                                );
                                            }
                                        }}
                                        disabled={!editContent.trim() || isStreaming}
                                    >
                                        {t("messageList.send")}
                                    </Button>
                                </Group>
                            </Box>
                        ) : (
                            <Paper
                                p="sm"
                                radius="md"
                                className={msg.role === "user" ? "user-message-bubble" : msg.role === "assistant" ? "assistant-message" : undefined}
                                style={{
                                    maxWidth: msg.role === "user" ? "70%" : compact ? "98%" : "95%",
                                    backgroundColor:
                                        msg.role === "user"
                                            ? "var(--mantine-color-blue-filled)"
                                            : "var(--mantine-color-default)",
                                    color: msg.role === "user" ? "var(--mantine-color-white)" : undefined,
                                }}
                            >
                                {msg.role === "user" ? (
                                    renderUserMessageBody(msg.content)
                                ) : msg.role === "assistant" && tryParseContentBlocks(msg.content) ? (
                                    renderUserMessageBody(msg.content)
                                ) : isStreaming &&
                                  index === messages.length - 1 &&
                                  !msg.content.trim() &&
                                  activeToolCalls.length === 0 ? (
                                    <Box
                                        className="typing-indicator"
                                        role="status"
                                        aria-live="polite"
                                        aria-label={t("chat.modelTyping")}
                                    >
                                        <span className="typing-indicator-dot" />
                                        <span className="typing-indicator-dot" />
                                        <span className="typing-indicator-dot" />
                                    </Box>
                                ) : (
                                    <Box
                                        className="markdown-body"
                                        style={{ fontSize: settings.font_size ?? 14 }}
                                    >
                                        <ReactMarkdown
                                            remarkPlugins={[remarkGfm]}
                                            rehypePlugins={[rehypeHighlight]}
                                            components={{
                                                pre: ({ children, ...props }) => (
                                                    <CodeBlock {...props}>
                                                        {children}
                                                    </CodeBlock>
                                                ),
                                            }}
                                        >
                                            {msg.content || (isStreaming && index === messages.length - 1 ? "▍" : "")}
                                        </ReactMarkdown>
                                    </Box>
                                )}
                                {isStreaming && index === messages.length - 1 && activeToolCalls.length > 0 && (
                                    <ToolCallBlock toolCalls={activeToolCalls} />
                                )}
                            </Paper>
                        )}
                    </Box>
                    <Box
                        className="message-actions"
                        style={{
                            display: "flex",
                            justifyContent: msg.role === "user" ? "flex-end" : "flex-start",
                            marginTop: 2,
                            gap: 2,
                        }}
                    >
                        {msg.role === "user" && !isStreaming && editingMessageId !== msg.id && (
                            <Tooltip label={t("messageList.edit")}>
                                <ActionIcon
                                    size="xs"
                                    variant="subtle"
                                    onClick={() => {
                                        setEditingMessageId(msg.id);
                                        setEditContent(msg.content);
                                    }}
                                    aria-label={t("messageList.edit")}
                                >
                                    <IconEdit size={14} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                        <Tooltip label={copiedMessageId === msg.id ? t("messageList.copied") : t("common.copy")}>
                            <ActionIcon
                                size="xs"
                                variant="subtle"
                                onClick={() =>
                                    handleCopyMessage(
                                        editingMessageId === msg.id ? editContent : msg.content,
                                        msg.id
                                    )
                                }
                                aria-label={copiedMessageId === msg.id ? t("messageList.copied") : t("common.copy")}
                            >
                                {copiedMessageId === msg.id ? (
                                    <IconCheck size={14} stroke={1.5} color="var(--mantine-color-green-6)" />
                                ) : (
                                    <IconCopy size={14} stroke={1.5} />
                                )}
                            </ActionIcon>
                        </Tooltip>
                        {msg.role === "assistant" && (msg.content.trim() || tryParseContentBlocks(msg.content)) && (
                            <Tooltip label={playingMessageId === msg.id ? t("chat.stopSpeaking") : t("chat.speak")}>
                                <ActionIcon
                                    size="xs"
                                    variant="subtle"
                                    onClick={() => {
                                        if (playingMessageId === msg.id) {
                                            stopTts();
                                        } else {
                                            const content = tryParseContentBlocks(msg.content)
                                                ? (() => {
                                                      try {
                                                          const blocks = JSON.parse(msg.content) as ContentBlock[];
                                                          return blocks
                                                              .filter((b): b is ContentBlockText => b.type === "text")
                                                              .map((b) => b.text)
                                                              .join("\n");
                                                      } catch {
                                                          return msg.content;
                                                      }
                                                  })()
                                                : msg.content;
                                            speakMessage(msg.id, content);
                                        }
                                    }}
                                    aria-label={playingMessageId === msg.id ? t("chat.stopSpeaking") : t("chat.speak")}
                                >
                                    {playingMessageId === msg.id ? (
                                        <IconPlayerStop size={14} stroke={1.5} />
                                    ) : (
                                        <IconVolume size={14} stroke={1.5} />
                                    )}
                                </ActionIcon>
                            </Tooltip>
                        )}
                    </Box>
                </Box>
            ))}
            <div ref={bottomRef} />
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
                        alt=""
                        style={{ maxWidth: "100%", height: "auto", display: "block" }}
                    />
                )}
            </Modal>
        </Box>
    );
}
import { useCallback, useEffect, useRef, useState } from "react";
import { ActionIcon, Box, Button, Group, Modal, Paper, Text, Textarea, Tooltip } from "@mantine/core";
import { IconCheck, IconCopy, IconEdit, IconFile, IconFileText } from "@tabler/icons-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { openPath } from "@tauri-apps/plugin-opener";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir, join } from "@tauri-apps/api/path";
import { useChatStore } from "../store/chatStore";
import { notify } from "../utils/notify";
import type { ContentBlock, Message } from "../types";

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
            <Tooltip label={copied ? "Скопировано" : "Копировать код"}>
                <ActionIcon
                    className="copy-button"
                    size="xs"
                    variant="subtle"
                    onClick={handleCopy}
                    style={{ position: "absolute", top: 8, right: 8 }}
                    aria-label={copied ? "Скопировано" : "Копировать код"}
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

export function MessageList({ messages, isStreaming, onEditResend, compact = false }: Props) {
    const { settings, scrollTargetId, setScrollTargetId } = useChatStore();
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
                            if (!appDataDirPath) return <Text key={i} size="xs" c="dimmed">Загрузка...</Text>;
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
                                            Открыть
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
        return (
            <Box
                style={{
                    flex: 1,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                }}
            >
                <Text c="dimmed" size="lg">
                    Начните диалог — напишите сообщение
                </Text>
            </Box>
        );
    }

    return (
        <Box style={{ flex: 1, overflowY: "auto" }} p={compact ? "xs" : "md"}>
            {messages.map((msg, index) => (
                <Box key={msg.id} id={msg.id} mb={compact ? "xs" : "md"}>
                    <Box
                        style={{
                            display: "flex",
                            justifyContent: msg.role === "user" ? "flex-end" : "flex-start",
                        }}
                    >
                        {msg.role === "user" && editingMessageId === msg.id ? (
                            <Box style={{ maxWidth: "75%", width: "100%" }}>
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
                                        Отмена
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
                                        Отправить
                                    </Button>
                                </Group>
                            </Box>
                        ) : (
                            <Paper
                                p="sm"
                                radius="md"
                                style={{
                                    maxWidth: msg.role === "user" ? "75%" : compact ? "98%" : "95%",
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
                                  !msg.content.trim() ? (
                                    <Box
                                        className="typing-indicator"
                                        role="status"
                                        aria-live="polite"
                                        aria-label="Модель печатает"
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
                            </Paper>
                        )}
                    </Box>
                    <Box
                        style={{
                            display: "flex",
                            justifyContent: msg.role === "user" ? "flex-end" : "flex-start",
                            marginTop: 2,
                            gap: 2,
                        }}
                    >
                        {msg.role === "user" && !isStreaming && editingMessageId !== msg.id && (
                            <Tooltip label="Редактировать">
                                <ActionIcon
                                    className="message-copy-btn"
                                    size="xs"
                                    variant="subtle"
                                    onClick={() => {
                                        setEditingMessageId(msg.id);
                                        setEditContent(msg.content);
                                    }}
                                    aria-label="Редактировать"
                                >
                                    <IconEdit size={14} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                        <Tooltip label={copiedMessageId === msg.id ? "Скопировано" : "Копировать"}>
                            <ActionIcon
                                className="message-copy-btn"
                                size="xs"
                                variant="subtle"
                                onClick={() =>
                                    handleCopyMessage(
                                        editingMessageId === msg.id ? editContent : msg.content,
                                        msg.id
                                    )
                                }
                                aria-label={copiedMessageId === msg.id ? "Скопировано" : "Копировать"}
                            >
                                {copiedMessageId === msg.id ? (
                                    <IconCheck size={14} stroke={1.5} color="var(--mantine-color-green-6)" />
                                ) : (
                                    <IconCopy size={14} stroke={1.5} />
                                )}
                            </ActionIcon>
                        </Tooltip>
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
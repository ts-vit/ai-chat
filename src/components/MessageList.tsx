import { useEffect, useRef, useState } from "react";
import { ActionIcon, Box, Button, Group, Paper, Text, Textarea, Tooltip } from "@mantine/core";
import { IconCheck, IconCopy, IconEdit } from "@tabler/icons-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { useChatStore } from "../store/chatStore";
import type { Message } from "../types";

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
    const { settings } = useChatStore();
    const bottomRef = useRef<HTMLDivElement>(null);
    const [copiedMessageId, setCopiedMessageId] = useState<string | null>(null);
    const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
    const [editContent, setEditContent] = useState("");

    // Автоскролл при новых сообщениях и стриминге
    useEffect(() => {
        bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [messages, isStreaming]);

    const handleCopyMessage = async (content: string, messageId: string) => {
        try {
            await navigator.clipboard.writeText(content);
            setCopiedMessageId(messageId);
            setTimeout(() => setCopiedMessageId(null), 2000);
        } catch {
            // clipboard API unavailable or denied
        }
    };

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
                                    <Text
                                        size="sm"
                                        style={{
                                            whiteSpace: "pre-wrap",
                                            fontSize: settings.font_size ?? 14,
                                        }}
                                    >
                                        {msg.content}
                                    </Text>
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
        </Box>
    );
}
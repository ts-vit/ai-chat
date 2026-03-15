import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionIcon, Badge, Box, Button, Code, Collapse, Group, Modal, Paper, Popover, Spoiler, Stack, Text, Textarea, ThemeIcon, Tooltip, UnstyledButton } from "@mantine/core";
import { IconBrain, IconCheck, IconChevronDown, IconChevronLeft, IconChevronRight, IconCopy, IconCurrencyDollar, IconEdit, IconFile, IconFileText, IconListCheck, IconLoader2, IconPackage, IconPlayerStop, IconTool, IconUsers, IconVolume, IconWorldSearch, IconX } from "@tabler/icons-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { openPath } from "@tauri-apps/plugin-opener";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-shell";
import { useChatStore } from "../store/chatStore";
import { notify } from "../utils/notify";
import { formatRelativeTime } from "../utils/formatDate";
import { ToolCallBlock } from "./ToolCallBlock";
import { WebSourcesBlock } from "./WebSourcesBlock";
import type { AgentTask, ContentBlock, ContentBlockText, MessageWithSiblings, WebSource } from "../types";

const SOURCE_LINK_PREFIX = "__source__";

const COMPACT_BUILTIN_TOOLS = new Set([
    'web_search', 'web_read',
    'memory_save', 'memory_search',
    'workspace_write', 'workspace_read', 'workspace_list',
    'spawn_agent', 'check_agent', 'get_agent_result', 'cancel_agent',
]);

interface CompactToolInfo {
    icon: typeof IconTool;
    text: string;
    color: string;
}

function getCompactToolInfo(
    toolName: string,
    resultText: string,
    argsStr: string,
    isError: boolean,
    t: (key: string, opts?: Record<string, unknown>) => string,
): CompactToolInfo {
    try {
        switch (toolName) {
            case 'web_search': {
                if (isError || /failed|no search results/i.test(resultText)) {
                    return { icon: IconWorldSearch, text: `${t('chat.toolResultWebSearch')} ${t('chat.toolResultWebSearchNoResults')}`, color: 'yellow' };
                }
                const count = (resultText.match(/^\d+\./gm) || []).length;
                return { icon: IconWorldSearch, text: `${t('chat.toolResultWebSearch')} ${t('chat.toolResultWebSearchResults', { count })}`, color: 'gray' };
            }
            case 'web_read': {
                let domain = '';
                try {
                    const args = JSON.parse(argsStr);
                    domain = new URL(args.url).hostname.replace(/^www\./, '');
                } catch { /* ignore */ }
                return { icon: IconFileText, text: domain ? t('chat.toolResultWebRead', { domain }) : t('chat.toolResultWebReadPage'), color: 'gray' };
            }
            case 'memory_save':
                return { icon: IconBrain, text: t('chat.toolResultMemorySave'), color: 'green' };
            case 'memory_search': {
                if (!resultText.trim() || /no memories found|nothing found/i.test(resultText)) {
                    return { icon: IconBrain, text: `${t('chat.toolResultMemorySearch')} ${t('chat.toolResultMemorySearchNoResults')}`, color: 'yellow' };
                }
                const count = (resultText.match(/^\d+\./gm) || []).length || resultText.split('\n').filter(l => l.trim()).length;
                return { icon: IconBrain, text: `${t('chat.toolResultMemorySearch')} ${t('chat.toolResultMemorySearchResults', { count })}`, color: 'gray' };
            }
            case 'workspace_write': {
                let name = '';
                try { name = JSON.parse(argsStr).name; } catch { /* ignore */ }
                return { icon: IconPackage, text: name ? t('chat.toolResultWorkspaceWrite', { name }) : t('chat.toolResultWorkspaceWrite', { name: 'artifact' }), color: 'green' };
            }
            case 'workspace_read': {
                let name = '';
                try { name = JSON.parse(argsStr).name; } catch { /* ignore */ }
                return { icon: IconPackage, text: name ? t('chat.toolResultWorkspaceRead', { name }) : t('chat.toolResultWorkspaceRead', { name: 'artifact' }), color: 'gray' };
            }
            case 'workspace_list': {
                const count = resultText.split('\n').filter(l => l.trim()).length;
                return { icon: IconPackage, text: t('chat.toolResultWorkspaceList', { count }), color: 'gray' };
            }
            case 'spawn_agent': {
                let goal = '';
                try { goal = JSON.parse(argsStr).goal || ''; } catch { /* ignore */ }
                if (goal.length > 60) goal = goal.slice(0, 60) + '…';
                return { icon: IconUsers, text: t('chat.toolResultSubAgentLaunched', { goal: goal || 'task' }), color: 'blue' };
            }
            case 'check_agent':
                return { icon: IconUsers, text: t('chat.toolResultSubAgentCheck'), color: 'blue' };
            case 'get_agent_result':
                return { icon: IconUsers, text: t('chat.toolResultSubAgentResult'), color: 'blue' };
            case 'cancel_agent':
                return { icon: IconUsers, text: t('chat.toolResultSubAgentCancelled'), color: 'yellow' };
        }
    } catch { /* fallback below */ }
    return { icon: IconTool, text: toolName, color: 'gray' };
}

function formatMessageCost(cost: number): string {
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    return `$${cost.toFixed(2)}`;
}

/** Replace [N], [N,M], [Source N] with markdown links that we intercept in custom `a` component. Only in non-code segments. */
function replaceSourceRefsInMarkdown(content: string, sourceCount: number): string {
    if (sourceCount <= 0) return content;
    const parts = content.split("```");
    const out: string[] = [];
    for (let i = 0; i < parts.length; i++) {
        if (i % 2 === 1) {
            out.push(parts[i]);
            continue;
        }
        let text = parts[i];
        // [N, M, K] — multiple in one bracket (do first so we don't match single [N] inside)
        text = text.replace(/\[(\d+)\s*,\s*(\d+)(?:\s*,\s*\d+)*\]/g, (match) => {
            const numbers = match
                .slice(1, -1)
                .split(/\s*,\s*/)
                .map((s) => parseInt(s.trim(), 10));
            return numbers
                .map((num) =>
                    num >= 1 && num <= sourceCount ? `[${num}](${SOURCE_LINK_PREFIX}${num}__)` : `[${num}]`
                )
                .join(" ");
        });
        // [N] — single (avoid matching [N] that is already part of [N](__source__N__))
        text = text.replace(/\[(\d+)\](?!\(__source__\1__\))/g, (_, n) => {
            const num = parseInt(n, 10);
            return num >= 1 && num <= sourceCount ? `[${num}](${SOURCE_LINK_PREFIX}${num}__)` : `[${n}]`;
        });
        // [Source N] — fallback
        text = text.replace(/\[Source\s+(\d+)\]/gi, (match, n) => {
            const num = parseInt(n, 10);
            return num >= 1 && num <= sourceCount ? `[${num}](${SOURCE_LINK_PREFIX}${num}__)` : match;
        });
        out.push(text);
    }
    return out.join("```");
}

function SourceRefBadge({
    n,
    sources,
}: {
    n: number;
    sources: WebSource[];
}) {
    const source = sources[n - 1];
    if (!source) return <>{n}</>;
    const handleClick = () => {
        open(source.url).catch(console.error);
    };
    return (
        <Tooltip label={source.title} withArrow>
            <span
                className="source-reference"
                role="button"
                tabIndex={0}
                onClick={handleClick}
                onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        handleClick();
                    }
                }}
                title={source.title}
            >
                {n}
            </span>
        </Tooltip>
    );
}

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
    messages: MessageWithSiblings[];
    isStreaming: boolean;
    onEditResend: (messageId: string, newContent: string) => Promise<void>;
    onSwitchBranch: (messageId: string) => Promise<void>;
    compact?: boolean;
}

const DENSITY_PADDING = { compact: "xs", standard: "sm", spacious: "md" } as const;
const DENSITY_MB = { compact: "xs", standard: "sm", spacious: "lg" } as const;

export function MessageList({ messages, isStreaming, onEditResend, onSwitchBranch, compact = false }: Props) {
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
    const [switching, setSwitching] = useState(false);
    const [expandedSubAgents, setExpandedSubAgents] = useState<Set<string>>(new Set());
    const [expandedPlanTasks, setExpandedPlanTasks] = useState<Set<string>>(new Set());
    const [showAllPlanSteps, setShowAllPlanSteps] = useState(false);
    const [expandedTools, setExpandedTools] = useState<Set<string>>(new Set());
    const activeSubAgents = useChatStore((s) => s.activeSubAgents);
    const currentAgentRun = useChatStore((s) => s.currentAgentRun);
    const activePlan = useChatStore((s) => s.activePlan);

    const handleSwitchBranch = useCallback(async (messageId: string) => {
        setSwitching(true);
        await onSwitchBranch(messageId);
        setTimeout(() => setSwitching(false), 150);
    }, [onSwitchBranch]);

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
        node.style.outline = "2px solid var(--mantine-color-brand-5)";
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
        (content: string, webSources?: WebSource[]) => {
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
            const hasSourceRefs = webSources && webSources.length > 0;
            return (
                <Box>
                    {blocks.map((block) => {
                        if (block.type === "image") {
                            if (!appDataDirPath) return <Text key={`image-${block.path}`} size="xs" c="dimmed">{t("common.loading")}</Text>;
                            const isAbsolute =
                                block.path.startsWith("/") || /^[A-Za-z]:[/\\]/.test(block.path);
                            const fullPath = isAbsolute
                                ? block.path
                                : base + sep + block.path.replace(/^[/\\]+/, "").replace(/\//g, sep);
                            const src = convertFileSrc(fullPath);
                            return (
                                <Box key={`image-${block.path}`} mb="xs">
                                    <img
                                        src={src}
                                        alt={block.name}
                                        style={{
                                            maxWidth: 400,
                                            borderRadius: "var(--mantine-radius-sm)",
                                            cursor: "pointer",
                                            display: "block",
                                        }}
                                        role="button"
                                        tabIndex={0}
                                        onClick={() => setLightboxSrc(src)}
                                        onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setLightboxSrc(src); } }}
                                    />
                                </Box>
                            );
                        }
                        if (block.type === "file") {
                            return (
                                <Paper key={`file-${block.path}`} p="xs" mb="xs" withBorder radius="sm">
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
                            const textContent =
                                hasSourceRefs && webSources
                                    ? replaceSourceRefsInMarkdown(block.text, webSources.length)
                                    : block.text;
                            return (
                                <Box
                                    key={`text-${block.text.slice(0, 32)}`}
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
                                            ...(hasSourceRefs && webSources
                                                ? {
                                                      a: ({
                                                          href,
                                                          children,
                                                          ...anchorProps
                                                      }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => {
                                                          if (href?.startsWith(SOURCE_LINK_PREFIX)) {
                                                              const numStr = href
                                                                  .slice(SOURCE_LINK_PREFIX.length)
                                                                  .replace(/__$/, "");
                                                              const n = parseInt(numStr, 10);
                                                              if (
                                                                  !Number.isNaN(n) &&
                                                                  n >= 1 &&
                                                                  n <= webSources.length
                                                              ) {
                                                                  return (
                                                                      <SourceRefBadge n={n} sources={webSources} />
                                                                  );
                                                              }
                                                          }
                                                          return (
                                                              <a href={href} {...anchorProps}>
                                                                  {children}
                                                              </a>
                                                          );
                                                      },
                                                  }
                                                : {}),
                                        }}
                                    >
                                        {textContent}
                                    </ReactMarkdown>
                                </Box>
                            );
                        }
                        return null;
                    })}
                </Box>
            );
        },
        [settings.font_size, appDataDirPath, handleOpenFile, t]
    );

    const toggleSubAgent = useCallback((runId: string) => {
        setExpandedSubAgents((prev) => {
            const next = new Set(prev);
            if (next.has(runId)) next.delete(runId);
            else next.add(runId);
            return next;
        });
    }, []);

    const togglePlanTask = useCallback((taskId: string) => {
        setExpandedPlanTasks((prev) => {
            const next = new Set(prev);
            if (next.has(taskId)) next.delete(taskId);
            else next.add(taskId);
            return next;
        });
    }, []);

    // Build set of known sub-agent run IDs
    const subAgentRunIds = new Set(Object.keys(activeSubAgents));
    const mainRunId = currentAgentRun?.id;

    // Build plan task run ID lookup
    const planTaskByRunId = useMemo(() => {
        const map = new Map<string, AgentTask>();
        if (activePlan) {
            for (const task of activePlan.tasks) {
                if (task.agentRunId) map.set(task.agentRunId, task);
            }
        }
        return map;
    }, [activePlan]);

    // Group messages: identify sub-agent and plan task message groups
    type MessageGroup =
        | { type: "normal"; msg: MessageWithSiblings; index: number }
        | { type: "subagent"; runId: string; goal: string; status: string; messages: { msg: MessageWithSiblings; index: number }[] }
        | { type: "plantask"; taskId: string; title: string; status: string; result: string | null; runId: string; taskIndex: number; messages: { msg: MessageWithSiblings; index: number }[] };

    const messageGroups: MessageGroup[] = [];
    let i = 0;
    while (i < messages.length) {
        const msg = messages[i];
        const runId = msg.agentRunId;
        if (runId && planTaskByRunId.has(runId)) {
            const task = planTaskByRunId.get(runId)!;
            const group: MessageGroup = {
                type: "plantask",
                taskId: task.id,
                title: task.title,
                status: task.status,
                result: task.result,
                runId,
                taskIndex: activePlan!.tasks.findIndex(t => t.id === task.id),
                messages: [],
            };
            while (i < messages.length && messages[i].agentRunId === runId) {
                group.messages.push({ msg: messages[i], index: i });
                i++;
            }
            messageGroups.push(group);
        } else if (runId && runId !== mainRunId && subAgentRunIds.has(runId)) {
            const subInfo = activeSubAgents[runId];
            const group: MessageGroup = {
                type: "subagent",
                runId,
                goal: subInfo?.goal ?? t("subAgent.title"),
                status: subInfo?.status ?? "running",
                messages: [],
            };
            while (i < messages.length && messages[i].agentRunId === runId) {
                group.messages.push({ msg: messages[i], index: i });
                i++;
            }
            messageGroups.push(group);
        } else {
            messageGroups.push({ type: "normal", msg, index: i });
            i++;
        }
    }

    if (messages.length === 0) {
        return <Box style={{ flex: 1 }} />;
    }

    const renderMessage = (msg: MessageWithSiblings, index: number) => (
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
                                            ? "var(--mantine-color-brand-filled)"
                                            : "var(--mantine-color-default)",
                                    color: msg.role === "user" ? "var(--mantine-color-white)" : undefined,
                                }}
                            >
                                {msg.agentStep != null && msg.role === "assistant" && (
                                    <Badge size="xs" variant="light" color="orange" mb={4}>
                                        {t("agent.step", { step: msg.agentStep })}
                                    </Badge>
                                )}
                                {msg.role === "tool" ? (
                                    (() => {
                                        let toolName = "tool";
                                        let resultText = msg.content;
                                        let argsStr = "";
                                        let isError = false;
                                        try {
                                            const parsed = JSON.parse(msg.content);
                                            if (parsed && typeof parsed === "object") {
                                                toolName = parsed.tool_name || "tool";
                                                resultText = parsed.result ?? msg.content;
                                                argsStr = parsed.arguments || "";
                                                isError = !!parsed.is_error;
                                            }
                                        } catch { /* use raw content as fallback */ }

                                        const isShort = resultText.length < 200;
                                        const isLikelyCode = resultText.includes('\n') && resultText.length > 200;

                                        const renderResultContent = () => {
                                            if (isShort) {
                                                return <Text size="sm" style={{ opacity: 0.85 }}>{resultText}</Text>;
                                            }
                                            if (isLikelyCode) {
                                                return (
                                                    <Spoiler maxHeight={200} showLabel={t('common.showMore')} hideLabel={t('common.showLess')}>
                                                        <Code block style={{ maxHeight: 400, overflowY: 'auto', fontSize: (settings.font_size ?? 14) - 1 }}>
                                                            {resultText}
                                                        </Code>
                                                    </Spoiler>
                                                );
                                            }
                                            return (
                                                <Spoiler maxHeight={200} showLabel={t('common.showMore')} hideLabel={t('common.showLess')}>
                                                    <Box className="markdown-body" style={{ fontSize: (settings.font_size ?? 14) - 1, maxHeight: 400, overflowY: 'auto' }}>
                                                        <ReactMarkdown
                                                            remarkPlugins={[remarkGfm]}
                                                            rehypePlugins={[rehypeHighlight]}
                                                            components={{
                                                                pre: ({ children, ...props }) => (
                                                                    <CodeBlock {...props}>{children}</CodeBlock>
                                                                ),
                                                            }}
                                                        >
                                                            {resultText}
                                                        </ReactMarkdown>
                                                    </Box>
                                                </Spoiler>
                                            );
                                        };

                                        if (COMPACT_BUILTIN_TOOLS.has(toolName)) {
                                            const info = getCompactToolInfo(toolName, resultText, argsStr, isError, t);
                                            const ToolIcon = info.icon;
                                            const toolKey = `${msg.id}_${toolName}`;
                                            const isExpanded = expandedTools.has(toolKey);
                                            return (
                                                <Box>
                                                    <Group
                                                        gap={6}
                                                        style={{ cursor: 'pointer' }}
                                                        onClick={() => setExpandedTools(prev => {
                                                            const next = new Set(prev);
                                                            if (next.has(toolKey)) next.delete(toolKey);
                                                            else next.add(toolKey);
                                                            return next;
                                                        })}
                                                    >
                                                        <ThemeIcon size="xs" variant="light" color={info.color} radius="sm">
                                                            <ToolIcon size={12} stroke={1.5} />
                                                        </ThemeIcon>
                                                        <Text size="xs" c="dimmed">{info.text}</Text>
                                                        <IconChevronDown
                                                            size={12}
                                                            color="var(--mantine-color-dimmed)"
                                                            style={{
                                                                transform: isExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
                                                                transition: 'transform 150ms',
                                                            }}
                                                        />
                                                    </Group>
                                                    <Collapse in={isExpanded}>
                                                        <Box mt={4} pl={8} style={{ borderLeft: '2px solid var(--mantine-color-default-border)' }}>
                                                            {renderResultContent()}
                                                        </Box>
                                                    </Collapse>
                                                </Box>
                                            );
                                        }

                                        return (
                                            <Box style={{ borderLeft: '3px solid var(--mantine-color-brand-3)', paddingLeft: 10 }}>
                                                <Group gap={6} mb={4}>
                                                    <IconTool size={14} stroke={1.5} color="var(--mantine-color-dimmed)" />
                                                    <Badge size="xs" variant="light" color={isError ? "red" : "gray"}>
                                                        {toolName}
                                                    </Badge>
                                                </Group>
                                                {renderResultContent()}
                                            </Box>
                                        );
                                    })()
                                ) : msg.role === "user" ? (
                                    renderUserMessageBody(msg.content)
                                ) : msg.role === "assistant" && tryParseContentBlocks(msg.content) ? (
                                    renderUserMessageBody(msg.content, msg.webSources)
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
                                                ...(msg.webSources && msg.webSources.length > 0
                                                    ? {
                                                          a: ({
                                                              href,
                                                              children,
                                                              ...anchorProps
                                                          }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => {
                                                              if (href?.startsWith(SOURCE_LINK_PREFIX)) {
                                                                  const numStr = href
                                                                      .slice(SOURCE_LINK_PREFIX.length)
                                                                      .replace(/__$/, "");
                                                                  const n = parseInt(numStr, 10);
                                                                  if (
                                                                      !Number.isNaN(n) &&
                                                                      n >= 1 &&
                                                                      n <= msg.webSources!.length
                                                                  ) {
                                                                      return (
                                                                          <SourceRefBadge
                                                                              n={n}
                                                                              sources={msg.webSources!}
                                                                          />
                                                                      );
                                                                  }
                                                              }
                                                              return (
                                                                  <a href={href} {...anchorProps}>
                                                                      {children}
                                                                  </a>
                                                              );
                                                          },
                                                      }
                                                    : {}),
                                            }}
                                        >
                                            {msg.webSources && msg.webSources.length > 0
                                                ? replaceSourceRefsInMarkdown(
                                                      msg.content ||
                                                          (isStreaming && index === messages.length - 1 ? "▍" : ""),
                                                      msg.webSources.length
                                                  )
                                                : msg.content || (isStreaming && index === messages.length - 1 ? "▍" : "")}
                                        </ReactMarkdown>
                                    </Box>
                                )}
                                {isStreaming && index === messages.length - 1 && activeToolCalls.length > 0 && (
                                    <ToolCallBlock toolCalls={activeToolCalls} />
                                )}
                                {msg.role === "assistant" && msg.webSources && msg.webSources.length > 0 && (
                                    <WebSourcesBlock sources={msg.webSources} />
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
                        {msg.role === "assistant" && msg.cost != null && msg.cost > 0 && (
                            <Tooltip label={t("budget.messageCost")}>
                                <Group gap={4} style={{ cursor: "default" }}>
                                    <IconCurrencyDollar size={14} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                                    <Text size="xs" c="dimmed">
                                        {formatMessageCost(msg.cost)}
                                    </Text>
                                </Group>
                            </Tooltip>
                        )}
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
                        {msg.siblingCount > 1 && (
                            <Group gap={2} wrap="nowrap" style={{ display: "inline-flex", alignItems: "center" }}>
                                <Tooltip label={t("chat.previousBranch")} withArrow>
                                    <ActionIcon
                                        size="xs"
                                        variant="subtle"
                                        color="gray"
                                        disabled={msg.siblingIndex === 0}
                                        onClick={() => handleSwitchBranch(msg.siblingIds[msg.siblingIndex - 1])}
                                    >
                                        <IconChevronLeft size={14} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                                <Popover width={320} position="bottom" shadow="md" withArrow>
                                    <Popover.Target>
                                        <Tooltip label={t("chat.branchList")} withArrow>
                                            <UnstyledButton>
                                                <Text size="xs" c="dimmed" style={{ userSelect: "none", minWidth: 24, textAlign: "center", cursor: "pointer" }}>
                                                    {msg.siblingIndex + 1}/{msg.siblingCount}
                                                </Text>
                                            </UnstyledButton>
                                        </Tooltip>
                                    </Popover.Target>
                                    <Popover.Dropdown p="xs">
                                        <Stack gap={4}>
                                            {msg.siblings.map((sibling) => (
                                                <UnstyledButton
                                                    key={sibling.id}
                                                    onClick={() => handleSwitchBranch(sibling.id)}
                                                    style={{
                                                        padding: "6px 8px",
                                                        borderRadius: "var(--mantine-radius-sm)",
                                                        backgroundColor: sibling.id === msg.id
                                                            ? "var(--mantine-color-brand-light)"
                                                            : "transparent",
                                                    }}
                                                >
                                                    <Group justify="space-between" wrap="nowrap">
                                                        <div style={{ flex: 1, minWidth: 0 }}>
                                                            {sibling.contentPreview ? (
                                                                <Text size="xs" fw={sibling.id === msg.id ? 600 : 400} truncate>
                                                                    {sibling.contentPreview}
                                                                </Text>
                                                            ) : (
                                                                <Text size="xs" c="dimmed" fs="italic">
                                                                    {t("chat.emptyMessage")}
                                                                </Text>
                                                            )}
                                                        </div>
                                                        <Text size="xs" c="dimmed" style={{ flexShrink: 0 }}>
                                                            {formatRelativeTime(sibling.createdAt)}
                                                        </Text>
                                                    </Group>
                                                </UnstyledButton>
                                            ))}
                                        </Stack>
                                    </Popover.Dropdown>
                                </Popover>
                                <Tooltip label={t("chat.nextBranch")} withArrow>
                                    <ActionIcon
                                        size="xs"
                                        variant="subtle"
                                        color="gray"
                                        disabled={msg.siblingIndex === msg.siblingCount - 1}
                                        onClick={() => handleSwitchBranch(msg.siblingIds[msg.siblingIndex + 1])}
                                    >
                                        <IconChevronRight size={14} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            </Group>
                        )}
                    </Box>
                </Box>
    );

    const planTaskStatusColor = (status: string) =>
        status === "completed" ? "teal" : status === "failed" ? "red" : status === "running" ? "blue" : "gray";

    const renderPlanTaskStatusIcon = (status: string) => {
        if (status === "running") return <IconLoader2 size={14} stroke={1.5} style={{ animation: "spin 1s linear infinite" }} />;
        if (status === "completed") return <IconCheck size={14} stroke={2} color="var(--mantine-color-teal-5)" />;
        if (status === "failed") return <IconX size={14} stroke={2} color="var(--mantine-color-red-5)" />;
        return <Box style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--mantine-color-dimmed)", margin: 3 }} />;
    };

    return (
        <Box style={{ flex: 1, overflowY: "auto", transition: "opacity 150ms ease", opacity: switching ? 0.6 : 1 }} p={listPadding}>
            {planTaskByRunId.size > 0 && (
                <Group justify="flex-end" mb={4}>
                    <Button
                        variant="subtle"
                        size="compact-xs"
                        leftSection={<IconChevronDown size={12} stroke={1.5} style={{ transform: showAllPlanSteps ? "rotate(0deg)" : "rotate(-90deg)", transition: "transform 150ms ease" }} />}
                        onClick={() => setShowAllPlanSteps(v => !v)}
                    >
                        {showAllPlanSteps ? t("planTask.hideAllSteps") : t("planTask.showAllSteps")}
                    </Button>
                </Group>
            )}
            {messageGroups.map((group) => {
                if (group.type === "normal") {
                    return renderMessage(group.msg, group.index);
                }
                if (group.type === "plantask") {
                    const isExpanded = showAllPlanSteps || expandedPlanTasks.has(group.taskId);
                    const color = planTaskStatusColor(group.status);
                    return (
                        <Box
                            key={`plantask-${group.taskId}`}
                            mb={messageMb}
                            style={{
                                border: "1px solid var(--mantine-color-default-border)",
                                borderRadius: "var(--mantine-radius-sm)",
                                overflow: "hidden",
                            }}
                        >
                            <Group
                                gap={6}
                                p={8}
                                wrap="nowrap"
                                style={{ cursor: "pointer", background: "var(--mantine-color-dark-6)" }}
                                onClick={() => togglePlanTask(group.taskId)}
                            >
                                <IconChevronDown
                                    size={14}
                                    stroke={1.5}
                                    style={{
                                        transform: isExpanded ? "rotate(0deg)" : "rotate(-90deg)",
                                        transition: "transform 150ms ease",
                                    }}
                                />
                                <IconListCheck size={14} stroke={1.5} style={{ opacity: 0.6 }} />
                                <Text size="xs" fw={500} style={{ flex: 1, minWidth: 0 }} lineClamp={1}>
                                    {t("planTask.label", { index: group.taskIndex + 1 })}: {group.title}
                                </Text>
                                <Badge size="xs" color={color} variant="light">
                                    {t(`plans.taskStatus.${group.status}`, group.status)}
                                </Badge>
                                <Text size="xs" c="dimmed">
                                    {group.messages.length}
                                </Text>
                            </Group>
                            {!isExpanded && group.result && (
                                <Box px={8} py={4} style={{ borderTop: "1px solid var(--mantine-color-default-border)" }}>
                                    <Text size="xs" c="dimmed" lineClamp={2}>
                                        {group.result}
                                    </Text>
                                </Box>
                            )}
                            <Collapse in={isExpanded}>
                                <Box p="xs">
                                    {group.messages.map(({ msg, index }) => renderMessage(msg, index))}
                                </Box>
                            </Collapse>
                        </Box>
                    );
                }
                // subagent group
                const isExpanded = expandedSubAgents.has(group.runId);
                const statusColor = group.status === "completed" ? "green" : group.status === "failed" ? "red" : group.status === "cancelled" ? "gray" : "orange";
                return (
                    <Box
                        key={`subagent-${group.runId}`}
                        mb={messageMb}
                        style={{
                            border: "1px solid var(--mantine-color-default-border)",
                            borderRadius: "var(--mantine-radius-sm)",
                            overflow: "hidden",
                        }}
                    >
                        <Group
                            gap={6}
                            p={8}
                            wrap="nowrap"
                            style={{ cursor: "pointer", background: "var(--mantine-color-dark-6)" }}
                            onClick={() => toggleSubAgent(group.runId)}
                        >
                            <IconChevronDown
                                size={14}
                                stroke={1.5}
                                style={{
                                    transform: isExpanded ? "rotate(0deg)" : "rotate(-90deg)",
                                    transition: "transform 150ms ease",
                                }}
                            />
                            <IconUsers size={14} stroke={1.5} style={{ opacity: 0.6 }} />
                            <Text size="xs" fw={500} style={{ flex: 1, minWidth: 0 }} lineClamp={1}>
                                {t("subAgent.title")}: {group.goal}
                            </Text>
                            <Badge size="xs" color={statusColor} variant="light">
                                {group.status}
                            </Badge>
                            <Text size="xs" c="dimmed">
                                {group.messages.length} {t("navSidebar.messages").toLowerCase()}
                            </Text>
                        </Group>
                        <Collapse in={isExpanded}>
                            <Box p="xs">
                                {group.messages.map(({ msg, index }) => renderMessage(msg, index))}
                            </Box>
                        </Collapse>
                    </Box>
                );
            })}
            {activePlan && (activePlan.plan.status === "completed" || activePlan.plan.status === "failed") && planTaskByRunId.size > 0 && (
                <Paper
                    p="sm"
                    mb={messageMb}
                    radius="sm"
                    style={{
                        border: `1px solid var(--mantine-color-${activePlan.plan.status === "completed" ? "teal" : "red"}-4)`,
                        background: `var(--mantine-color-${activePlan.plan.status === "completed" ? "teal" : "red"}-light)`,
                    }}
                >
                    <Group gap="xs" mb={4}>
                        {activePlan.plan.status === "completed"
                            ? <IconCheck size={16} stroke={2} color="var(--mantine-color-teal-5)" />
                            : <IconX size={16} stroke={2} color="var(--mantine-color-red-5)" />
                        }
                        <Text size="sm" fw={600}>
                            {activePlan.plan.status === "completed" ? t("plans.planCompleted") : t("plans.planFailed")}
                        </Text>
                    </Group>
                    <Text size="xs" c="dimmed" mb={4}>{activePlan.plan.goal}</Text>
                    <Text size="xs" c="dimmed">
                        {t("plans.progress", {
                            completed: activePlan.tasks.filter(t2 => t2.status === "completed").length,
                            total: activePlan.tasks.length,
                        })}
                    </Text>
                    <Stack gap={2} mt={8}>
                        {activePlan.tasks.map((task, idx) => (
                            <Group key={task.id} gap={4} wrap="nowrap">
                                {renderPlanTaskStatusIcon(task.status)}
                                <Text size="xs" lineClamp={1} style={{ flex: 1 }}>
                                    {idx + 1}. {task.title}
                                </Text>
                            </Group>
                        ))}
                    </Stack>
                </Paper>
            )}
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
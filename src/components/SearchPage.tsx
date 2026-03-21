import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Badge,
    Box,
    Button,
    Group,
    Loader,
    Progress,
    ScrollArea,
    SegmentedControl,
    Stack,
    Text,
    TextInput,
    Title,
} from "@mantine/core";
import { IconDatabase, IconFileText, IconRobot, IconSearch, IconUser } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useChatStore } from "../store/chatStore";
import { formatRelativeDate } from "../utils/formatDate";
import type { IndexingStatus, KbDocSearchResult, Message, SearchResult } from "../types";

const DEBOUNCE_MS = 500;
const PREVIEW_MAX = 200;
const SNIPPET_MAX_LEN = 130;

interface DbMessageResponse {
    id: string;
    chatId: string;
    role: string;
    content: string;
    parentId?: string;
    timestamp: number;
    model?: string;
    promptTokens?: number;
    completionTokens?: number;
    cost?: number;
    has_attachments?: number;
}

function dbMessageToMessage(m: DbMessageResponse): Message {
    return {
        id: m.id,
        role: m.role as Message["role"],
        content: m.content,
        parentId: m.parentId,
        timestamp: m.timestamp,
        model: m.model,
        promptTokens: m.promptTokens,
        completionTokens: m.completionTokens,
        cost: m.cost,
        hasAttachments: m.has_attachments ? true : undefined,
    };
}

function truncateSnippet(text: string, maxLen: number): string {
    const plain = text.replace(/\s+/g, " ").trim();
    if (plain.length <= maxLen) return plain;
    const cut = plain.slice(0, maxLen);
    const lastSpace = cut.lastIndexOf(" ");
    const end = lastSpace > maxLen / 2 ? lastSpace : maxLen;
    return cut.slice(0, end).trim() + "…";
}

function extractPlainText(content: string): string {
    const trimmed = content.trimStart();
    if (!trimmed.startsWith("[")) return content;
    try {
        const parsed = JSON.parse(content) as unknown;
        if (!Array.isArray(parsed)) return content;
        const textParts: string[] = [];
        for (const b of parsed as { type?: string; text?: string }[]) {
            if (b?.type === "text" && typeof b.text === "string") textParts.push(b.text);
        }
        return textParts.length > 0 ? textParts.join(" ") : content;
    } catch {
        return content;
    }
}

function groupByChat(results: SearchResult[]): { chatId: string; chatTitle: string; results: SearchResult[] }[] {
    const byChat = new Map<string, SearchResult[]>();
    for (const r of results) {
        const list = byChat.get(r.chatId) ?? [];
        list.push(r);
        byChat.set(r.chatId, list);
    }
    return Array.from(byChat.entries()).map(([chatId, list]) => ({
        chatId,
        chatTitle: list[0]?.chatTitle ?? "",
        results: list,
    }));
}

interface DocGroup {
    kbId: string;
    kbName: string;
    documents: {
        documentId: string;
        documentName: string;
        chunks: KbDocSearchResult[];
    }[];
}

export function groupDocResults(results: KbDocSearchResult[]): DocGroup[] {
    const byKb = new Map<string, KbDocSearchResult[]>();
    for (const r of results) {
        const list = byKb.get(r.kbId) ?? [];
        list.push(r);
        byKb.set(r.kbId, list);
    }
    return Array.from(byKb.entries()).map(([kbId, kbResults]) => {
        const byDoc = new Map<string, KbDocSearchResult[]>();
        for (const r of kbResults) {
            const list = byDoc.get(r.documentId) ?? [];
            list.push(r);
            byDoc.set(r.documentId, list);
        }
        return {
            kbId,
            kbName: kbResults[0]?.kbName ?? "",
            documents: Array.from(byDoc.entries()).map(([documentId, chunks]) => ({
                documentId,
                documentName: chunks[0]?.documentName ?? "",
                chunks,
            })),
        };
    });
}

export function SearchPage() {
    const { t } = useTranslation();
    const { setView, setActiveChat, setScrollTargetId } = useChatStore();
    const [query, setQuery] = useState("");
    const [debouncedQuery, setDebouncedQuery] = useState("");
    const [results, setResults] = useState<SearchResult[]>([]);
    const [loading, setLoading] = useState(false);
    const [segment, setSegment] = useState<"all" | "chats" | "docs">("all");
    const [indexingStatus, setIndexingStatus] = useState<IndexingStatus>({
        indexed: 0,
        total: 0,
        inProgress: false,
    });
    const [messagesByChatId, setMessagesByChatId] = useState<Record<string, Message[]>>({});
    const [docResults, setDocResults] = useState<KbDocSearchResult[]>([]);
    const [docLoading, setDocLoading] = useState(false);

    useEffect(() => {
        const t = setTimeout(() => setDebouncedQuery(query.trim()), DEBOUNCE_MS);
        return () => clearTimeout(t);
    }, [query]);

    const fetchStatus = useCallback(async () => {
        try {
            const status = await invoke<IndexingStatus>("get_indexing_status");
            setIndexingStatus(status);
        } catch {
            // ignore
        }
    }, []);

    useEffect(() => {
        fetchStatus();
        const unlistenProgress = listen<{ indexed: number; total: number }>(
            "indexing-progress",
            (e) => {
                setIndexingStatus((s) => ({
                    ...s,
                    indexed: e.payload?.indexed ?? s.indexed,
                    total: e.payload?.total ?? s.total,
                    inProgress: true,
                }));
            }
        );
        const unlistenDone = listen("indexing-done", () => {
            setIndexingStatus((s) => ({ ...s, inProgress: false }));
            fetchStatus();
        });
        return () => {
            unlistenProgress.then((fn) => fn());
            unlistenDone.then((fn) => fn());
        };
    }, [fetchStatus]);

    useEffect(() => {
        if (!debouncedQuery || segment === "docs") {
            setResults([]);
            setLoading(false);
            return;
        }
        setLoading(true);
        invoke<SearchResult[]>("search_messages", { query: debouncedQuery })
            .then((data) => setResults(Array.isArray(data) ? data : []))
            .catch((e) => {
                console.error("Chat search error:", e);
                setResults([]);
            })
            .finally(() => setLoading(false));
    }, [debouncedQuery, segment]);

    useEffect(() => {
        if (!debouncedQuery || segment === "chats") {
            setDocResults([]);
            setDocLoading(false);
            return;
        }
        setDocLoading(true);
        invoke<KbDocSearchResult[]>("search_all_knowledge_bases", { query: debouncedQuery, topK: 20 })
            .then((data) => setDocResults(Array.isArray(data) ? data : []))
            .catch((e) => {
                console.error("Doc search error:", e);
                setDocResults([]);
            })
            .finally(() => setDocLoading(false));
    }, [debouncedQuery, segment]);

    useEffect(() => {
        if (results.length === 0) {
            setMessagesByChatId({});
            return;
        }
        const chatIds = Array.from(new Set(results.map((r) => r.chatId)));
        let cancelled = false;
        Promise.all(
            chatIds.map(async (chatId) => {
                try {
                    const rows = await invoke<DbMessageResponse[]>("get_messages", { chatId });
                    return [chatId, rows.map(dbMessageToMessage)] as const;
                } catch {
                    return [chatId, []] as const;
                }
            })
        ).then((pairs) => {
            if (cancelled) return;
            setMessagesByChatId((prev) => ({
                ...prev,
                ...Object.fromEntries(pairs),
            }));
        });
        return () => {
            cancelled = true;
        };
    }, [results]);

    const groups = useMemo(() => groupByChat(results), [results]);
    const docGroups = useMemo(() => groupDocResults(docResults), [docResults]);

    const getContextSnippet = useCallback(
        (chatId: string, messageId: string, role: string): string | null => {
            const messages = messagesByChatId[chatId];
            if (!messages?.length) return null;
            const idx = messages.findIndex((m) => m.id === messageId);
            if (idx < 0) return null;
            if (role === "user") {
                const next = messages.slice(idx + 1).find((m) => m.role === "assistant");
                if (!next) return null;
                return truncateSnippet(extractPlainText(next.content), SNIPPET_MAX_LEN);
            }
            if (role === "assistant") {
                const prev = [...messages.slice(0, idx)].reverse().find((m) => m.role === "user");
                if (!prev) return null;
                return truncateSnippet(extractPlainText(prev.content), SNIPPET_MAX_LEN);
            }
            return null;
        },
        [messagesByChatId]
    );

    const handleMessageClick = useCallback(
        (chatId: string, messageId: string) => {
            setActiveChat(chatId);
            setScrollTargetId(messageId);
            setView("chat");
        },
        [setActiveChat, setScrollTargetId, setView]
    );

    const handleDocChunkClick = useCallback(
        (_chunk: KbDocSearchResult) => {
            setView("knowledgeBases");
        },
        [setView]
    );

    const isLoading = loading || docLoading;

    return (
        <Box
            style={{
                height: "100vh",
                display: "flex",
                flexDirection: "column",
                overflow: "hidden",
            }}
        >
            <Group
                p="md"
                gap="sm"
                style={{
                    flexShrink: 0,
                    borderBottom: "1px solid var(--mantine-color-default-border)",
                }}
            >
                <Button variant="subtle" onClick={() => setView("chat")}>
                    ← {t("common.back")}
                </Button>
                <Title order={2}>{t("search.title")}</Title>
            </Group>

            <Box
                style={{
                    flex: 1,
                    display: "flex",
                    flexDirection: "column",
                    minHeight: 0,
                }}
            >
                <Box
                    style={{
                        flex: 1,
                        display: "flex",
                        flexDirection: "column",
                        minHeight: 0,
                        maxWidth: 720,
                        width: "100%",
                        margin: "0 auto",
                    }}
                >
                    <Stack p="md" gap="md" style={{ flexShrink: 0 }}>
                        <TextInput
                            placeholder={t("search.placeholder")}
                            leftSection={<IconSearch size={18} stroke={1.5} />}
                            value={query}
                            onChange={(e) => setQuery(e.currentTarget.value)}
                            autoFocus
                        />
                        <SegmentedControl
                            value={segment}
                            onChange={(v) => setSegment(v as "all" | "chats" | "docs")}
                            data={[
                                { value: "all", label: t("search.segmentAll") },
                                { value: "chats", label: t("search.segmentChats") },
                                { value: "docs", label: t("search.segmentDocs") },
                            ]}
                        />
                        {indexingStatus.inProgress && (
                            <Box>
                                <Text size="sm" c="dimmed" mb={4}>
                                    {t("search.indexing", { indexed: indexingStatus.indexed, total: indexingStatus.total })}
                                </Text>
                                <Progress
                                    value={
                                        indexingStatus.total > 0
                                            ? (indexingStatus.indexed / indexingStatus.total) * 100
                                            : 0
                                    }
                                    size="sm"
                                />
                            </Box>
                        )}
                    </Stack>

                    <ScrollArea style={{ flex: 1, minHeight: 0 }} type="scroll">
                        <Stack p="md" gap="lg">
                            {isLoading && (
                                <Group justify="center" py="xl">
                                    <Loader size="sm" />
                                </Group>
                            )}
                            {!isLoading && !debouncedQuery && (
                                <Stack align="center" gap="sm" py="xl">
                                    <IconSearch size={48} stroke={1.5} style={{ opacity: 0.5 }} />
                                    <Text c="dimmed">{t("search.noQuery")}</Text>
                                </Stack>
                            )}
                            {!isLoading && debouncedQuery && results.length === 0 && docResults.length === 0 && (
                                <Text c="dimmed" ta="center" py="xl">
                                    {segment === "docs"
                                        ? t("search.noDocResults", { query: debouncedQuery })
                                        : t("search.noResults", { query: debouncedQuery })}
                                </Text>
                            )}

                            {/* Chat results */}
                            {!loading && groups.length > 0 && segment !== "docs" && (
                                <>
                                    {segment === "all" && docGroups.length > 0 && (
                                        <Text fw={600} size="sm" c="dimmed">{t("search.chatsSection")}</Text>
                                    )}
                                    {groups.map((g) => (
                                        <Box key={g.chatId}>
                                            <Button
                                                variant="subtle"
                                                size="compact-sm"
                                                style={{
                                                    fontWeight: 500,
                                                    color: "var(--mantine-color-dimmed)",
                                                    fontSize: "var(--mantine-font-size-xs)",
                                                    marginBottom: 8,
                                                }}
                                                onClick={() => {
                                                    setActiveChat(g.chatId);
                                                    setScrollTargetId(null);
                                                    setView("chat");
                                                }}
                                            >
                                                {g.chatTitle || t("search.noTitle")}
                                            </Button>
                                            <Stack gap="xs">
                                                {g.results.map((r) => {
                                                    const snippet = getContextSnippet(r.chatId, r.messageId, r.role);
                                                    return (
                                                        <Box
                                                            key={r.messageId}
                                                            style={{
                                                                padding: 12,
                                                                borderRadius: 8,
                                                                border: "1px solid var(--mantine-color-default-border)",
                                                                cursor: "pointer",
                                                            }}
                                                            onClick={() => handleMessageClick(r.chatId, r.messageId)}
                                                        >
                                                            <Group justify="space-between" wrap="nowrap" align="flex-start">
                                                                <Group wrap="nowrap" gap="sm" style={{ minWidth: 0, flex: 1 }}>
                                                                    {r.role === "user" ? (
                                                                        <IconUser size={18} stroke={1.5} style={{ flexShrink: 0 }} />
                                                                    ) : (
                                                                        <IconRobot size={18} stroke={1.5} style={{ flexShrink: 0 }} />
                                                                    )}
                                                                    <Text size="sm" lineClamp={2} style={{ minWidth: 0 }}>
                                                                        {r.content.length > PREVIEW_MAX
                                                                            ? `${r.content.slice(0, PREVIEW_MAX)}...`
                                                                            : r.content}
                                                                    </Text>
                                                                </Group>
                                                                <Badge size="sm" variant="light">
                                                                    {r.score.toFixed(2)}
                                                                </Badge>
                                                            </Group>
                                                            <Text size="xs" c="dimmed" mt={4}>
                                                                {formatRelativeDate(r.timestamp)}
                                                            </Text>
                                                            {snippet && (
                                                                <Text size="xs" c="dimmed" lineClamp={2} mt="xs">
                                                                    {snippet}
                                                                </Text>
                                                            )}
                                                        </Box>
                                                    );
                                                })}
                                            </Stack>
                                        </Box>
                                    ))}
                                </>
                            )}

                            {/* Document results */}
                            {!docLoading && docGroups.length > 0 && segment !== "chats" && (
                                <>
                                    {segment === "all" && groups.length > 0 && (
                                        <Text fw={600} size="sm" c="dimmed" mt="md">{t("search.documentsSection")}</Text>
                                    )}
                                    {segment === "docs" && (
                                        <Text fw={600} size="sm" c="dimmed">{t("search.documentsSection")}</Text>
                                    )}
                                    {docGroups.map((kbGroup) => (
                                        <Box key={kbGroup.kbId}>
                                            <Group gap="xs" mb={8}>
                                                <IconDatabase size={16} stroke={1.5} style={{ opacity: 0.6 }} />
                                                <Text size="xs" c="dimmed" fw={500}>{kbGroup.kbName}</Text>
                                            </Group>
                                            {kbGroup.documents.map((doc) => (
                                                <Box key={doc.documentId} mb="sm">
                                                    <Text size="xs" c="dimmed" fw={500} mb={4} ml={24}>
                                                        {doc.documentName}
                                                    </Text>
                                                    <Stack gap="xs">
                                                        {doc.chunks.map((chunk) => (
                                                            <Box
                                                                key={chunk.chunkId}
                                                                style={{
                                                                    padding: 12,
                                                                    borderRadius: 8,
                                                                    border: "1px solid var(--mantine-color-default-border)",
                                                                    cursor: "pointer",
                                                                    marginLeft: 24,
                                                                }}
                                                                onClick={() => handleDocChunkClick(chunk)}
                                                            >
                                                                <Group justify="space-between" wrap="nowrap" align="flex-start">
                                                                    <Group wrap="nowrap" gap="sm" style={{ minWidth: 0, flex: 1 }}>
                                                                        <IconFileText size={18} stroke={1.5} style={{ flexShrink: 0 }} />
                                                                        <Text size="sm" lineClamp={3} style={{ minWidth: 0 }}>
                                                                            {chunk.content.length > PREVIEW_MAX
                                                                                ? `${chunk.content.slice(0, PREVIEW_MAX)}...`
                                                                                : chunk.content}
                                                                        </Text>
                                                                    </Group>
                                                                    <Badge size="sm" variant="light" color="orange">
                                                                        {chunk.score.toFixed(2)}
                                                                    </Badge>
                                                                </Group>
                                                                {chunk.headingHierarchy && (
                                                                    <Text size="xs" c="dimmed" mt={4}>
                                                                        {chunk.headingHierarchy}
                                                                    </Text>
                                                                )}
                                                            </Box>
                                                        ))}
                                                    </Stack>
                                                </Box>
                                            ))}
                                        </Box>
                                    ))}
                                </>
                            )}
                        </Stack>
                    </ScrollArea>
                </Box>
            </Box>
        </Box>
    );
}

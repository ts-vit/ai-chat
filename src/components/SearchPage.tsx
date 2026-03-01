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
import { IconRobot, IconSearch, IconUser } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useChatStore } from "../store/chatStore";
import { formatRelativeDate } from "../utils/formatDate";
import type { IndexingStatus, SearchResult } from "../types";
import { notify } from "../utils/notify";

const DEBOUNCE_MS = 500;
const PREVIEW_MAX = 200;

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
        if (!debouncedQuery) {
            setResults([]);
            setLoading(false);
            return;
        }
        setLoading(true);
        invoke<SearchResult[]>("search_messages", { query: debouncedQuery })
            .then((data) => setResults(Array.isArray(data) ? data : []))
            .catch((e) => {
                notify.error(String(e));
                setResults([]);
            })
            .finally(() => setLoading(false));
    }, [debouncedQuery]);

    const groups = useMemo(() => groupByChat(results), [results]);

    const handleMessageClick = useCallback(
        (chatId: string, messageId: string) => {
            setActiveChat(chatId);
            setScrollTargetId(messageId);
            setView("chat");
        },
        [setActiveChat, setScrollTargetId, setView]
    );

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
                        { value: "docs", label: t("search.segmentDocs"), disabled: true },
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
                    {loading && (
                        <Group justify="center" py="xl">
                            <Loader size="sm" />
                        </Group>
                    )}
                    {!loading && !debouncedQuery && (
                        <Stack align="center" gap="sm" py="xl">
                            <IconSearch size={48} stroke={1.5} style={{ opacity: 0.5 }} />
                            <Text c="dimmed">{t("search.noQuery")}</Text>
                        </Stack>
                    )}
                    {!loading && debouncedQuery && results.length === 0 && (
                        <Text c="dimmed" ta="center" py="xl">
                            {t("search.noResults", { query: debouncedQuery })}
                        </Text>
                    )}
                    {!loading && groups.length > 0 && (
                        <>
                            {groups.map((g) => (
                                <Box key={g.chatId}>
                                    <Button
                                        variant="subtle"
                                        size="compact-sm"
                                        style={{ fontWeight: 600, marginBottom: 8 }}
                                        onClick={() => {
                                            setActiveChat(g.chatId);
                                            setScrollTargetId(null);
                                            setView("chat");
                                        }}
                                    >
                                        {g.chatTitle || t("search.noTitle")}
                                    </Button>
                                    <Stack gap="xs">
                                        {g.results.map((r) => (
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
                                            </Box>
                                        ))}
                                    </Stack>
                                </Box>
                            ))}
                        </>
                    )}
                </Stack>
            </ScrollArea>
        </Box>
    );
}

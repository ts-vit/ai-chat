import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Badge,
    Box,
    Button,
    Group,
    Loader,
    NumberInput,
    ScrollArea,
    Select,
    Stack,
    Switch,
    Table,
    Text,
    TextInput,
} from "@mantine/core";
import { IconSearch } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "../../utils/notify";
import type { ModelCatalogEntry } from "../../types";

const CATEGORIES = ["coding", "writing", "analysis", "general", "fast", "vision"] as const;
const PROVIDERS = ["openrouter", "ollama", "custom"] as const;
const INITIAL_VISIBLE = 50;
const SHOW_MORE_STEP = 50;

interface ModelCatalogSectionProps {
    lastSync?: number | null;
    onCatalogUpdated?: () => void;
}

export function ModelCatalogSection({ lastSync, onCatalogUpdated }: ModelCatalogSectionProps) {
    const { t } = useTranslation();
    const [catalog, setCatalog] = useState<ModelCatalogEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [providerFilter, setProviderFilter] = useState<string | null>(null);
    const [searchQuery, setSearchQuery] = useState("");
    const [visibleCount, setVisibleCount] = useState(INITIAL_VISIBLE);
    const [updatingId, setUpdatingId] = useState<string | null>(null);

    const loadCatalog = useCallback(async () => {
        setLoading(true);
        try {
            const list = await invoke<ModelCatalogEntry[]>("get_model_catalog");
            setCatalog(list ?? []);
            setVisibleCount(INITIAL_VISIBLE);
        } catch (e) {
            notify.error(String(e));
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        loadCatalog();
    }, [loadCatalog, lastSync]);

    const filtered = useMemo(() => {
        let list = catalog;
        if (providerFilter) {
            list = list.filter((e) => e.provider === providerFilter);
        }
        if (searchQuery.trim()) {
            const q = searchQuery.trim().toLowerCase();
            list = list.filter((e) => e.displayName.toLowerCase().includes(q));
        }
        return [...list].sort((a, b) => a.displayName.localeCompare(b.displayName));
    }, [catalog, providerFilter, searchQuery]);

    const visible = useMemo(() => filtered.slice(0, visibleCount), [filtered, visibleCount]);
    const hasMore = visibleCount < filtered.length;

    const updateEntry = useCallback(
        async (
            id: string,
            patch: {
                category?: string;
                costPerInputToken?: number;
                costPerOutputToken?: number;
                isAvailable?: boolean;
            }
        ) => {
            setUpdatingId(id);
            try {
                await invoke("update_model_catalog_entry", {
                    id,
                    category: patch.category ?? undefined,
                    costPerInputToken: patch.costPerInputToken ?? undefined,
                    costPerOutputToken: patch.costPerOutputToken ?? undefined,
                    isAvailable: patch.isAvailable ?? undefined,
                });
                setCatalog((prev) =>
                    prev.map((e) => (e.id === id ? { ...e, ...patch } : e))
                );
                onCatalogUpdated?.();
            } catch (e) {
                notify.error(String(e));
            } finally {
                setUpdatingId(null);
            }
        },
        [onCatalogUpdated]
    );

    const providerBadgeColor = (provider: string) => {
        if (provider === "openrouter") return "blue";
        if (provider === "ollama") return "green";
        if (provider === "custom") return "orange";
        return "gray";
    };

    const renderCostCell = (
        perToken: number | undefined,
        id: string,
        field: "costPerInputToken" | "costPerOutputToken"
    ) => {
        const perM = (perToken ?? 0) * 1_000_000;
        return (
            <Box style={{ minWidth: 90 }}>
                <NumberInput
                    size="xs"
                    value={perToken != null ? perM : ""}
                    onChange={(v) => {
                        const num = typeof v === "string" ? parseFloat(v) : v;
                        if (num != null && !Number.isNaN(num)) {
                            updateEntry(id, { [field]: num / 1_000_000 });
                        }
                    }}
                    min={0}
                    step={0.01}
                    decimalScale={4}
                    prefix="$"
                    suffix={t("routing.perMillion")}
                    placeholder="—"
                    styles={{ input: { minHeight: 28 } }}
                />
            </Box>
        );
    };

    if (loading) {
        return (
            <Box py="md">
                <Loader size="sm" />
            </Box>
        );
    }

    if (catalog.length === 0) {
        return (
            <Text size="sm" c="dimmed" py="md">
                {t("routing.noModels")}
            </Text>
        );
    }

    return (
        <Stack gap="sm">
            <Group gap="sm">
                <Select
                    size="xs"
                    placeholder={t("common.search")}
                    data={[
                        { value: "", label: "All" },
                        ...PROVIDERS.map((p) => ({ value: p, label: p })),
                    ]}
                    value={providerFilter ?? ""}
                    onChange={(v) => setProviderFilter(v || null)}
                    style={{ width: 140 }}
                />
                <TextInput
                    size="xs"
                    placeholder={t("common.search")}
                    leftSection={<IconSearch size={14} stroke={1.5} />}
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.currentTarget.value)}
                    style={{ flex: 1, maxWidth: 220 }}
                />
            </Group>
            <ScrollArea h={400} type="auto" style={{ border: "1px solid var(--mantine-color-default-border)", borderRadius: "var(--mantine-radius-md)" }}>
                <Table striped="even" withTableBorder withColumnBorders>
                    <Table.Thead>
                        <Table.Tr>
                            <Table.Th>{t("common.name")}</Table.Th>
                            <Table.Th>{t("routing.category")}</Table.Th>
                            <Table.Th>Provider</Table.Th>
                            <Table.Th>Cost In</Table.Th>
                            <Table.Th>Cost Out</Table.Th>
                            <Table.Th>Available</Table.Th>
                        </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                        {visible.map((entry) => (
                            <Table.Tr key={entry.id}>
                                <Table.Td>
                                    <Text size="sm" truncate style={{ maxWidth: 180 }}>
                                        {entry.displayName}
                                    </Text>
                                </Table.Td>
                                <Table.Td>
                                    <Select
                                        size="xs"
                                        data={CATEGORIES.map((c) => ({ value: c, label: t(`routing.${c}`) }))}
                                        value={entry.category}
                                        onChange={(v) => v && updateEntry(entry.id, { category: v })}
                                        styles={{ input: { minHeight: 28 } }}
                                    />
                                </Table.Td>
                                <Table.Td>
                                    <Badge size="sm" variant="light" color={providerBadgeColor(entry.provider)}>
                                        {entry.provider}
                                    </Badge>
                                </Table.Td>
                                <Table.Td>
                                    {renderCostCell(entry.costPerInputToken, entry.id, "costPerInputToken")}
                                </Table.Td>
                                <Table.Td>
                                    {renderCostCell(entry.costPerOutputToken, entry.id, "costPerOutputToken")}
                                </Table.Td>
                                <Table.Td>
                                    <Switch
                                        size="sm"
                                        checked={entry.isAvailable}
                                        onChange={(e) =>
                                            updateEntry(entry.id, { isAvailable: e.currentTarget.checked })
                                        }
                                        disabled={updatingId === entry.id}
                                    />
                                </Table.Td>
                            </Table.Tr>
                        ))}
                    </Table.Tbody>
                </Table>
            </ScrollArea>
            {hasMore && (
                <Button variant="subtle" size="xs" onClick={() => setVisibleCount((n) => n + SHOW_MORE_STEP)}>
                    {t("routing.showMore")}
                </Button>
            )}
        </Stack>
    );
}

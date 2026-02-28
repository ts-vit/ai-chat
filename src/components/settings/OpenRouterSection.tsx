import { useEffect, useMemo, useState } from "react";
import {
    Accordion,
    Button,
    Checkbox,
    Group,
    Loader,
    Pagination,
    PasswordInput,
    SegmentedControl,
    Stack,
    Table,
    Text,
    TextInput,
    Tooltip,
} from "@mantine/core";
import { IconSearch } from "@tabler/icons-react";
import { useChatStore } from "../../store/chatStore";

const POPULAR_PREFIXES = [
    "anthropic/",
    "openai/",
    "google/",
    "meta-llama/",
    "deepseek/",
    "mistralai/",
];
const PAGE_SIZE = 20;

type FilterSegment = "all" | "popular" | "free" | "selected";
type SortKey = "name" | "prompt" | "completion" | "context";

function formatContextLength(n: number): string {
    if (n >= 1_000_000) return `${Math.round(n / 1_000_000)}M`;
    if (n >= 1000) return `${Math.round(n / 1000)}K`;
    return String(n);
}

function formatPrice(priceStr: string): string {
    const perM = parseFloat(priceStr) * 1_000_000;
    return `$${perM.toFixed(2)}`;
}

export interface OpenRouterSectionProps {
    apiKey: string;
    onApiKeyChange: (key: string) => void;
    managementKey: string;
    onManagementKeyChange: (key: string) => void;
    openrouterEnabledModels: string[];
    onOpenrouterEnabledModelsChange: (ids: string[]) => void;
}

export function OpenRouterSection({
    apiKey,
    onApiKeyChange,
    managementKey,
    onManagementKeyChange,
    openrouterEnabledModels,
    onOpenrouterEnabledModelsChange,
}: OpenRouterSectionProps) {
    const { models, modelsLoading, modelsError, loadModels } = useChatStore();

    const [filterSegment, setFilterSegment] = useState<FilterSegment>("all");
    const [searchQuery, setSearchQuery] = useState("");
    const [sortKey, setSortKey] = useState<SortKey | null>(null);
    const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
    const [page, setPage] = useState(1);

    useEffect(() => {
        if (models.length === 0 && !modelsLoading) {
            loadModels();
        }
    }, [models.length, modelsLoading, loadModels]);

    useEffect(() => {
        setPage(1);
    }, [filterSegment, searchQuery, sortKey, sortDir]);

    const filteredModels = useMemo(() => {
        let list = models;

        if (filterSegment === "popular") {
            list = list.filter((m) =>
                POPULAR_PREFIXES.some((p) => m.id.startsWith(p))
            );
        } else if (filterSegment === "free") {
            list = list.filter(
                (m) =>
                    parseFloat(m.pricing.prompt) === 0 &&
                    parseFloat(m.pricing.completion) === 0
            );
        } else if (filterSegment === "selected") {
            list = list.filter((m) => openrouterEnabledModels.includes(m.id));
        }

        const q = searchQuery.trim().toLowerCase();
        if (q) {
            list = list.filter(
                (m) =>
                    m.name.toLowerCase().includes(q) ||
                    m.id.toLowerCase().includes(q)
            );
        }

        if (sortKey) {
            list = [...list].sort((a, b) => {
                let va: string | number;
                let vb: string | number;
                switch (sortKey) {
                    case "name":
                        va = a.name.toLowerCase();
                        vb = b.name.toLowerCase();
                        break;
                    case "prompt":
                        va = parseFloat(a.pricing.prompt);
                        vb = parseFloat(b.pricing.prompt);
                        break;
                    case "completion":
                        va = parseFloat(a.pricing.completion);
                        vb = parseFloat(b.pricing.completion);
                        break;
                    case "context":
                        va = a.context_length;
                        vb = b.context_length;
                        break;
                    default:
                        return 0;
                }
                const cmp = va < vb ? -1 : va > vb ? 1 : 0;
                return sortDir === "asc" ? cmp : -cmp;
            });
        }

        return list;
    }, [models, filterSegment, searchQuery, sortKey, sortDir, openrouterEnabledModels]);

    const totalPages = Math.max(1, Math.ceil(filteredModels.length / PAGE_SIZE));
    const pageModels = useMemo(() => {
        const start = (page - 1) * PAGE_SIZE;
        return filteredModels.slice(start, start + PAGE_SIZE);
    }, [filteredModels, page]);

    const handleSort = (key: SortKey) => {
        if (sortKey === key) {
            setSortDir((d) => (d === "asc" ? "desc" : "asc"));
        } else {
            setSortKey(key);
            setSortDir("asc");
        }
    };

    const sortArrow = (key: SortKey) => {
        if (sortKey !== key) return null;
        return sortDir === "asc" ? " ↑" : " ↓";
    };

    const toggleOpenrouterModel = (modelId: string) => {
        const has = openrouterEnabledModels.includes(modelId);
        if (has) {
            onOpenrouterEnabledModelsChange(openrouterEnabledModels.filter((id) => id !== modelId));
        } else if (openrouterEnabledModels.length < 5) {
            onOpenrouterEnabledModelsChange([...openrouterEnabledModels, modelId]);
        }
    };

    return (
        <Stack gap="lg">
            <Accordion variant="separated">
                <Accordion.Item value="api-key">
                    <Accordion.Control>API-ключ</Accordion.Control>
                    <Accordion.Panel>
                        <PasswordInput
                            placeholder="sk-or-..."
                            value={apiKey}
                            onChange={(e) => onApiKeyChange(e.currentTarget.value)}
                        />
                    </Accordion.Panel>
                </Accordion.Item>
                <Accordion.Item value="management-key">
                    <Accordion.Control>Management key</Accordion.Control>
                    <Accordion.Panel>
                        <Stack gap="xs">
                            <PasswordInput
                                placeholder="sk-or-..."
                                value={managementKey}
                                onChange={(e) => onManagementKeyChange(e.currentTarget.value)}
                            />
                            <Text size="xs" c="dimmed">
                                Создайте на openrouter.ai/settings/keys с галочкой Management key (для отображения баланса)
                            </Text>
                        </Stack>
                    </Accordion.Panel>
                </Accordion.Item>
            </Accordion>

            <Stack gap="xs">
                <Group justify="space-between" wrap="nowrap">
                    <Text size="sm" fw={500}>
                        Модели OpenRouter (показывать при создании чата)
                    </Text>
                    <Text size="xs" c="dimmed">
                        Выбрано: {openrouterEnabledModels.length}/5
                    </Text>
                </Group>
                <Group wrap="nowrap" align="flex-end" gap="sm">
                    <SegmentedControl
                        value={filterSegment}
                        onChange={(v) => setFilterSegment(v as FilterSegment)}
                        data={[
                            { label: "Все", value: "all" },
                            { label: "Популярные", value: "popular" },
                            { label: "Бесплатные", value: "free" },
                            { label: "Выбранные", value: "selected" },
                        ]}
                    />
                    <TextInput
                        placeholder="Поиск по имени или id..."
                        leftSection={<IconSearch size={16} stroke={1.5} />}
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.currentTarget.value)}
                        style={{ flex: 1, minWidth: 200 }}
                    />
                </Group>

                {modelsError && (
                    <Group gap="sm">
                        <Text size="sm" c="red">
                            {modelsError}
                        </Text>
                        <Button
                            variant="light"
                            size="xs"
                            onClick={() => loadModels(true)}
                        >
                            Повторить
                        </Button>
                    </Group>
                )}

                {modelsLoading && (
                    <Group justify="center" py="xl">
                        <Loader />
                    </Group>
                )}

                {!modelsLoading && !modelsError &&
                    filterSegment === "selected" &&
                    filteredModels.length === 0 && (
                        <Text size="sm" c="dimmed" py="md">
                            Нет выбранных моделей
                        </Text>
                    )}

                {!modelsLoading &&
                    !modelsError &&
                    !(filterSegment === "selected" && filteredModels.length === 0) && (
                    <Table
                        withTableBorder
                        withColumnBorders
                        striped
                        highlightOnHover
                    >
                        <Table.Thead>
                            <Table.Tr>
                                <Table.Th style={{ width: 50 }} />
                                <Table.Th
                                    style={{ cursor: "pointer", width: "auto" }}
                                    onClick={() => handleSort("name")}
                                >
                                    Модель{sortArrow("name")}
                                </Table.Th>
                                <Table.Th style={{ width: 280, maxWidth: 300 }}>
                                    Описание
                                </Table.Th>
                                <Table.Th
                                    style={{ cursor: "pointer", width: 110 }}
                                    onClick={() => handleSort("prompt")}
                                >
                                    Вход{sortArrow("prompt")}
                                </Table.Th>
                                <Table.Th
                                    style={{ cursor: "pointer", width: 110 }}
                                    onClick={() => handleSort("completion")}
                                >
                                    Выход{sortArrow("completion")}
                                </Table.Th>
                                <Table.Th
                                    style={{ cursor: "pointer", width: 110 }}
                                    onClick={() => handleSort("context")}
                                >
                                    Контекст{sortArrow("context")}
                                </Table.Th>
                            </Table.Tr>
                        </Table.Thead>
                        <Table.Tbody>
                            {pageModels.map((m) => {
                                const checked = openrouterEnabledModels.includes(m.id);
                                const disabled = !checked && openrouterEnabledModels.length >= 5;
                                return (
                                    <Table.Tr
                                        key={m.id}
                                        style={{ cursor: disabled ? "not-allowed" : "pointer" }}
                                        onClick={() => !disabled && toggleOpenrouterModel(m.id)}
                                    >
                                        <Table.Td>
                                            <Checkbox
                                                checked={checked}
                                                disabled={disabled}
                                                onChange={() => toggleOpenrouterModel(m.id)}
                                                onClick={(e) => e.stopPropagation()}
                                                aria-label={m.name}
                                            />
                                        </Table.Td>
                                        <Table.Td>
                                            <Text size="sm">{m.name}</Text>
                                            <Text size="xs" c="dimmed">
                                                {m.id}
                                            </Text>
                                        </Table.Td>
                                        <Table.Td
                                            style={{
                                                maxWidth: 300,
                                                minWidth: 0,
                                            }}
                                        >
                                            {m.description?.trim() ? (
                                                <Tooltip
                                                    label={m.description}
                                                    multiline
                                                    maw={400}
                                                >
                                                    <Text
                                                        size="xs"
                                                        c="dimmed"
                                                        style={{
                                                            display: "-webkit-box",
                                                            WebkitLineClamp: 3,
                                                            WebkitBoxOrient:
                                                                "vertical",
                                                            overflow: "hidden",
                                                        }}
                                                    >
                                                        {m.description}
                                                    </Text>
                                                </Tooltip>
                                            ) : (
                                                <Text size="xs" c="dimmed">
                                                    —
                                                </Text>
                                            )}
                                        </Table.Td>
                                        <Table.Td>
                                            <Text size="sm">
                                                {formatPrice(m.pricing.prompt)}
                                            </Text>
                                        </Table.Td>
                                        <Table.Td>
                                            <Text size="sm">
                                                {formatPrice(m.pricing.completion)}
                                            </Text>
                                        </Table.Td>
                                        <Table.Td>
                                            <Text size="sm">
                                                {formatContextLength(m.context_length)}
                                            </Text>
                                        </Table.Td>
                                    </Table.Tr>
                                );
                            })}
                        </Table.Tbody>
                    </Table>
                )}

                {!modelsLoading &&
                    !modelsError &&
                    !(filterSegment === "selected" && filteredModels.length === 0) &&
                    totalPages > 1 && (
                    <Group justify="center" mt="sm">
                        <Pagination
                            total={totalPages}
                            value={page}
                            onChange={setPage}
                        />
                    </Group>
                )}
            </Stack>
        </Stack>
    );
}

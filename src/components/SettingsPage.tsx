import { useEffect, useMemo, useState } from "react";
import {
    ActionIcon,
    Box,
    Button,
    Tooltip,
    Card,
    Checkbox,
    Group,
    Loader,
    Modal,
    NumberInput,
    Pagination,
    PasswordInput,
    Radio,
    ScrollArea,
    SegmentedControl,
    Slider,
    Stack,
    Table,
    Text,
    Textarea,
    TextInput,
    Title,
} from "@mantine/core";
import { IconEdit, IconSearch, IconStar, IconStarFilled, IconTrash } from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import type { Preset } from "../types";
import { ConfirmModal } from "./ConfirmModal";

const POPULAR_PREFIXES = [
    "anthropic/",
    "openai/",
    "google/",
    "meta-llama/",
    "deepseek/",
    "mistralai/",
];
const PAGE_SIZE = 20;

type FilterSegment = "all" | "popular" | "free";
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

export function SettingsPage() {
    const {
        settings,
        saveSettings,
        setView,
        models,
        modelsLoading,
        modelsError,
        loadModels,
        presets,
        createPreset,
        updatePreset,
        deletePreset,
    } = useChatStore();

    const [apiKey, setApiKey] = useState(settings.api_key);
    const [managementKey, setManagementKey] = useState(settings.management_key);
    const [model, setModel] = useState(settings.model);
    const [temperature, setTemperature] = useState(settings.temperature);
    const [maxTokens, setMaxTokens] = useState(settings.max_tokens);
    const [fontSize, setFontSize] = useState(settings.font_size);

    const [filterSegment, setFilterSegment] = useState<FilterSegment>("all");
    const [searchQuery, setSearchQuery] = useState("");
    const [sortKey, setSortKey] = useState<SortKey | null>(null);
    const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
    const [page, setPage] = useState(1);

    const [presetModalOpen, setPresetModalOpen] = useState(false);
    const [presetEditId, setPresetEditId] = useState<string | null>(null);
    const [presetName, setPresetName] = useState("");
    const [presetContent, setPresetContent] = useState("");
    const [presetIsDefault, setPresetIsDefault] = useState(false);
    const [deletingPresetId, setDeletingPresetId] = useState<string | null>(null);

    useEffect(() => {
        setApiKey(settings.api_key);
        setManagementKey(settings.management_key);
        setModel(settings.model);
        setTemperature(settings.temperature);
        setMaxTokens(settings.max_tokens);
        setFontSize(settings.font_size);
    }, [settings]);

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
    }, [models, filterSegment, searchQuery, sortKey, sortDir]);

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

    const selectedModel = models.find((m) => m.id === model);

    const handleSave = async () => {
        await saveSettings({
            api_key: apiKey,
            management_key: managementKey,
            model,
            temperature,
            max_tokens: maxTokens,
            font_size: fontSize,
        });
        setView("chat");
    };

    const openPresetModal = (preset?: Preset) => {
        if (preset) {
            setPresetEditId(preset.id);
            setPresetName(preset.name);
            setPresetContent(preset.content);
            setPresetIsDefault(preset.isDefault);
        } else {
            setPresetEditId(null);
            setPresetName("");
            setPresetContent("");
            setPresetIsDefault(false);
        }
        setPresetModalOpen(true);
    };

    const closePresetModal = () => {
        setPresetModalOpen(false);
        setPresetEditId(null);
        setPresetName("");
        setPresetContent("");
        setPresetIsDefault(false);
    };

    const savePresetFromModal = async () => {
        const name = presetName.trim();
        const content = presetContent.trim();
        if (!name) return;
        if (presetEditId) {
            await updatePreset(presetEditId, name, content, presetIsDefault);
        } else {
            await createPreset(name, content, presetIsDefault);
        }
        closePresetModal();
    };

    const setPresetAsDefault = (id: string) => {
        const p = presets.find((x) => x.id === id);
        if (p) updatePreset(p.id, p.name, p.content, true);
    };

    return (
        <Box
            style={{
                height: "100vh",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                overflow: "hidden",
            }}
        >
            <ScrollArea
                style={{ flex: 1, width: "100%", maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}
                type="scroll"
            >
                <Stack p="lg" gap="lg">
                    {/* Шапка */}
                    <Group justify="space-between" align="center">
                        <Group>
                            <Button
                                variant="subtle"
                                onClick={() => setView("chat")}
                            >
                                ← Назад
                            </Button>
                            <Title order={2}>Настройки</Title>
                        </Group>
                    </Group>

                    {/* API */}
                    <Stack gap="xs">
                        <Text size="sm" fw={500}>
                            API
                        </Text>
                        <PasswordInput
                            placeholder="sk-or-..."
                            value={apiKey}
                            onChange={(e) => setApiKey(e.currentTarget.value)}
                        />
                    </Stack>

                    {/* Management key */}
                    <Stack gap="xs">
                        <Text size="sm" fw={500}>
                            Management-ключ OpenRouter (для отображения баланса)
                        </Text>
                        <PasswordInput
                            placeholder="sk-or-..."
                            value={managementKey}
                            onChange={(e) => setManagementKey(e.currentTarget.value)}
                        />
                        <Text size="xs" c="dimmed">
                            Создайте на openrouter.ai/settings/keys с галочкой Management key
                        </Text>
                    </Stack>

                    {/* Модель */}
                    <Stack gap="xs">
                        <Text size="sm" fw={500}>
                            Модель{selectedModel ? `: ${selectedModel.name}` : ""}
                        </Text>
                        <Group wrap="nowrap" align="flex-end" gap="sm">
                            <SegmentedControl
                                value={filterSegment}
                                onChange={(v) =>
                                    setFilterSegment(v as FilterSegment)
                                }
                                data={[
                                    { label: "Все", value: "all" },
                                    { label: "Популярные", value: "popular" },
                                    { label: "Бесплатные", value: "free" },
                                ]}
                            />
                            <TextInput
                                placeholder="Поиск по имени или id..."
                                leftSection={<IconSearch size={16} stroke={1.5} />}
                                value={searchQuery}
                                onChange={(e) =>
                                    setSearchQuery(e.currentTarget.value)
                                }
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

                        {!modelsLoading && !modelsError && (
                            <Radio.Group value={model} onChange={setModel}>
                                <Table
                                    withTableBorder
                                    withColumnBorders
                                    striped
                                    highlightOnHover
                                >
                                    <colgroup>
                                        <col style={{ width: 50 }} />
                                        <col />
                                        <col style={{ width: 110 }} />
                                        <col style={{ width: 110 }} />
                                        <col style={{ width: 110 }} />
                                    </colgroup>
                                    <Table.Thead>
                                        <Table.Tr>
                                            <Table.Th />
                                            <Table.Th
                                                style={{ cursor: "pointer" }}
                                                onClick={() => handleSort("name")}
                                            >
                                                Модель{sortArrow("name")}
                                            </Table.Th>
                                            <Table.Th
                                                style={{ cursor: "pointer" }}
                                                onClick={() => handleSort("prompt")}
                                            >
                                                Вход{sortArrow("prompt")}
                                            </Table.Th>
                                            <Table.Th
                                                style={{ cursor: "pointer" }}
                                                onClick={() => handleSort("completion")}
                                            >
                                                Выход{sortArrow("completion")}
                                            </Table.Th>
                                            <Table.Th
                                                style={{ cursor: "pointer" }}
                                                onClick={() => handleSort("context")}
                                            >
                                                Контекст{sortArrow("context")}
                                            </Table.Th>
                                        </Table.Tr>
                                    </Table.Thead>
                                    <Table.Tbody>
                                        {pageModels.map((m) => (
                                            <Table.Tr
                                                key={m.id}
                                                style={{ cursor: "pointer" }}
                                                onClick={() => setModel(m.id)}
                                            >
                                                <Table.Td>
                                                    <Radio value={m.id} />
                                                </Table.Td>
                                                <Table.Td>
                                                    <Text size="sm">{m.name}</Text>
                                                    <Text size="xs" c="dimmed">
                                                        {m.id}
                                                    </Text>
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
                                        ))}
                                    </Table.Tbody>
                                </Table>
                            </Radio.Group>
                        )}

                        {!modelsLoading && !modelsError && totalPages > 1 && (
                            <Group justify="center" mt="sm">
                                <Pagination
                                    total={totalPages}
                                    value={page}
                                    onChange={setPage}
                                />
                            </Group>
                        )}
                    </Stack>

                    {/* Пресеты */}
                    <Stack gap="xs">
                        <Group justify="space-between" align="center">
                            <Text size="sm" fw={500}>
                                Системные промпты (пресеты)
                            </Text>
                            <Button size="xs" variant="light" onClick={() => openPresetModal()}>
                                + Добавить пресет
                            </Button>
                        </Group>
                        {presets.length === 0 ? (
                            <Text size="xs" c="dimmed">
                                Нет пресетов. Добавьте пресет, чтобы задавать поведение модели для чатов.
                            </Text>
                        ) : (
                            <Stack gap="xs">
                                {presets.map((p) => (
                                    <Card key={p.id} withBorder padding="sm">
                                        <Group justify="space-between" wrap="nowrap" align="flex-start">
                                            <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
                                                <Text size="sm" fw={p.isDefault ? 700 : 400}>
                                                    {p.name}
                                                </Text>
                                                <Text size="xs" c="dimmed" lineClamp={1}>
                                                    {p.content.slice(0, 100)}
                                                    {p.content.length > 100 ? "…" : ""}
                                                </Text>
                                            </Stack>
                                            <Group gap="sm" wrap="nowrap">
                                                <Tooltip label="Редактировать">
                                                    <ActionIcon
                                                        variant="subtle"
                                                        size="xs"
                                                        onClick={() => openPresetModal(p)}
                                                        aria-label="Редактировать"
                                                    >
                                                        <IconEdit size={14} stroke={1.5} />
                                                    </ActionIcon>
                                                </Tooltip>
                                                <Tooltip label="Удалить">
                                                    <ActionIcon
                                                        variant="subtle"
                                                        size="xs"
                                                        color="red"
                                                        onClick={() => setDeletingPresetId(p.id)}
                                                        aria-label="Удалить"
                                                    >
                                                        <IconTrash size={14} stroke={1.5} />
                                                    </ActionIcon>
                                                </Tooltip>
                                                <Tooltip label={p.isDefault ? "По умолчанию" : "Сделать по умолчанию"}>
                                                    <ActionIcon
                                                        variant="subtle"
                                                        size="xs"
                                                        onClick={() => setPresetAsDefault(p.id)}
                                                        aria-label={p.isDefault ? "По умолчанию" : "Сделать по умолчанию"}
                                                    >
                                                        {p.isDefault ? (
                                                            <IconStarFilled size={14} stroke={1.5} />
                                                        ) : (
                                                            <IconStar size={14} stroke={1.5} />
                                                        )}
                                                    </ActionIcon>
                                                </Tooltip>
                                            </Group>
                                        </Group>
                                    </Card>
                                ))}
                            </Stack>
                        )}
                        <Modal
                            title={presetEditId ? "Редактировать пресет" : "Добавить пресет"}
                            opened={presetModalOpen}
                            onClose={closePresetModal}
                        >
                            <Stack gap="sm">
                                <TextInput
                                    label="Название"
                                    placeholder="Название пресета"
                                    value={presetName}
                                    onChange={(e) => setPresetName(e.currentTarget.value)}
                                />
                                <Textarea
                                    label="Системный промпт"
                                    placeholder="Текст, задающий поведение модели..."
                                    value={presetContent}
                                    onChange={(e) => setPresetContent(e.currentTarget.value)}
                                    minRows={3}
                                    maxRows={10}
                                    autosize
                                />
                                <Checkbox
                                    label="Использовать по умолчанию"
                                    checked={presetIsDefault}
                                    onChange={(e) => setPresetIsDefault(e.currentTarget.checked)}
                                />
                                <Group justify="flex-end" gap="sm">
                                    <Button variant="subtle" onClick={closePresetModal}>
                                        Отмена
                                    </Button>
                                    <Button onClick={savePresetFromModal} disabled={!presetName.trim()}>
                                        Сохранить
                                    </Button>
                                </Group>
                            </Stack>
                        </Modal>
                        <ConfirmModal
                            opened={deletingPresetId !== null}
                            onClose={() => setDeletingPresetId(null)}
                            onConfirm={() => {
                                if (deletingPresetId) {
                                    deletePreset(deletingPresetId);
                                    setDeletingPresetId(null);
                                }
                            }}
                            message="Пресет будет удалён безвозвратно."
                        />
                    </Stack>

                    {/* Параметры генерации */}
                    <Stack gap="xs">
                        <Text size="sm" fw={500}>
                            Параметры генерации
                        </Text>
                        <Group align="flex-end" wrap="nowrap">
                            <Box style={{ flex: 1 }}>
                                <Text size="sm" mb={4}>
                                    Температура: {temperature}
                                </Text>
                                <Slider
                                    min={0}
                                    max={2}
                                    step={0.1}
                                    value={temperature}
                                    onChange={setTemperature}
                                />
                            </Box>
                            <NumberInput
                                label="Макс. токенов"
                                value={maxTokens}
                                onChange={(val) =>
                                    setMaxTokens(Number(val) || 4096)
                                }
                                min={1}
                                max={200000}
                                style={{ width: 140 }}
                            />
                        </Group>
                    </Stack>

                    {/* Интерфейс */}
                    <Stack gap="xs">
                        <Text size="sm" fw={500}>
                            Размер шрифта сообщений
                        </Text>
                        <Text size="sm" mb={4}>
                            {fontSize}px
                        </Text>
                        <Slider
                            min={12}
                            max={24}
                            step={1}
                            value={fontSize}
                            onChange={setFontSize}
                            marks={[
                                { value: 12, label: "12" },
                                { value: 14, label: "14" },
                                { value: 16, label: "16" },
                                { value: 18, label: "18" },
                                { value: 20, label: "20" },
                                { value: 24, label: "24" },
                            ]}
                        />
                    </Stack>

                    {/* Сохранить */}
                    <Group justify="flex-end">
                        <Button onClick={handleSave}>Сохранить</Button>
                    </Group>
                </Stack>
            </ScrollArea>
        </Box>
    );
}

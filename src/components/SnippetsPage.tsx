import { useMemo, useState } from "react";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Card,
    Group,
    Modal,
    Pagination,
    ScrollArea,
    Select,
    Stack,
    Text,
    Textarea,
    TextInput,
    Title,
    Tooltip,
} from "@mantine/core";
import { IconEdit, IconSearch, IconTrash } from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import type { Category, Snippet } from "../types";
import { ConfirmModal } from "./ConfirmModal";
import { VariablesModal, getUniqueVariableNames } from "./VariablesModal";

const PREVIEW_MAX = 100;
const SNIPPET_PAGE_SIZE = 20;
const VARIABLE_REGEX = /\{([^}]+)\}/g;

function ContentPreview({ content, maxLen = PREVIEW_MAX }: { content: string; maxLen?: number }) {
    const truncated = content.length > maxLen ? content.slice(0, maxLen) + "…" : content;
    const parts: { text: string; isVar: boolean }[] = [];
    let lastIndex = 0;
    let m: RegExpExecArray | null;
    const re = new RegExp(VARIABLE_REGEX.source, "g");
    while ((m = re.exec(truncated)) !== null) {
        if (m.index > lastIndex) {
            parts.push({ text: truncated.slice(lastIndex, m.index), isVar: false });
        }
        parts.push({ text: m[0], isVar: true });
        lastIndex = m.index + m[0].length;
    }
    if (lastIndex < truncated.length) {
        parts.push({ text: truncated.slice(lastIndex), isVar: false });
    }
    if (parts.length === 0) parts.push({ text: truncated, isVar: false });

    return (
        <Text size="xs" c="dimmed" lineClamp={1}>
            {parts.map((p, i) =>
                p.isVar ? (
                    <Text key={i} span c="blue" inherit>
                        {p.text}
                    </Text>
                ) : (
                    <span key={i}>{p.text}</span>
                )
            )}
        </Text>
    );
}

function VariablePreview({ content }: { content: string }) {
    const parts: { text: string; isVar: boolean }[] = [];
    let lastIndex = 0;
    let m: RegExpExecArray | null;
    const re = new RegExp(VARIABLE_REGEX.source, "g");
    while ((m = re.exec(content)) !== null) {
        if (m.index > lastIndex) {
            parts.push({ text: content.slice(lastIndex, m.index), isVar: false });
        }
        parts.push({ text: m[0], isVar: true });
        lastIndex = m.index + m[0].length;
    }
    if (lastIndex < content.length) {
        parts.push({ text: content.slice(lastIndex), isVar: false });
    }
    if (parts.length === 0) parts.push({ text: content, isVar: false });

    return (
        <Text size="sm" c="dimmed">
            {parts.map((p, i) =>
                p.isVar ? (
                    <Text key={i} span c="blue" inherit>
                        {p.text}
                    </Text>
                ) : (
                    <span key={i}>{p.text}</span>
                )
            )}
        </Text>
    );
}

export function SnippetsPage() {
    const {
        categories,
        snippets,
        setView,
        createCategory,
        updateCategory,
        deleteCategory,
        createSnippet,
        updateSnippet,
        deleteSnippet,
        setInsertSnippetText,
    } = useChatStore();

    const [categoryFilter, setCategoryFilter] = useState<string>("all");
    const [searchQuery, setSearchQuery] = useState("");
    const [snippetPage, setSnippetPage] = useState(1);

    const [categoryModalOpen, setCategoryModalOpen] = useState(false);
    const [categoryEditId, setCategoryEditId] = useState<string | null>(null);
    const [categoryName, setCategoryName] = useState("");
    const [deletingCategoryId, setDeletingCategoryId] = useState<string | null>(null);

    const [snippetModalOpen, setSnippetModalOpen] = useState(false);
    const [snippetEditId, setSnippetEditId] = useState<string | null>(null);
    const [snippetName, setSnippetName] = useState("");
    const [snippetContent, setSnippetContent] = useState("");
    const [snippetCategoryId, setSnippetCategoryId] = useState<string | null>(null);
    const [deletingSnippetId, setDeletingSnippetId] = useState<string | null>(null);

    const [variablesModalOpen, setVariablesModalOpen] = useState(false);
    const [variablesModalContent, setVariablesModalContent] = useState("");
    const [variablesModalSnippetResolve, setVariablesModalSnippetResolve] = useState<
        ((text: string) => void) | null
    >(null);

    const categorySelectData = useMemo(
        () => [
            { value: "all", label: "Все" },
            ...categories.map((c) => ({ value: c.id, label: c.name })),
        ],
        [categories]
    );

    const filteredSnippets = useMemo(() => {
        let list = snippets;
        if (categoryFilter !== "all") {
            list = list.filter((s) => s.categoryId === categoryFilter);
        }
        const q = searchQuery.trim().toLowerCase();
        if (q) {
            list = list.filter(
                (s) =>
                    s.name.toLowerCase().includes(q) ||
                    s.content.toLowerCase().includes(q)
            );
        }
        return list;
    }, [snippets, categoryFilter, searchQuery]);

    const totalSnippetPages = Math.max(
        1,
        Math.ceil(filteredSnippets.length / SNIPPET_PAGE_SIZE)
    );
    const pageSnippets = useMemo(() => {
        const start = (snippetPage - 1) * SNIPPET_PAGE_SIZE;
        return filteredSnippets.slice(start, start + SNIPPET_PAGE_SIZE);
    }, [filteredSnippets, snippetPage]);

    const getCategoryName = (id: string) => categories.find((c) => c.id === id)?.name ?? id;

    const openCategoryModal = (cat?: Category) => {
        if (cat) {
            setCategoryEditId(cat.id);
            setCategoryName(cat.name);
        } else {
            setCategoryEditId(null);
            setCategoryName("");
        }
        setCategoryModalOpen(true);
    };

    const closeCategoryModal = () => {
        setCategoryModalOpen(false);
        setCategoryEditId(null);
        setCategoryName("");
    };

    const saveCategoryFromModal = async () => {
        const name = categoryName.trim();
        if (!name) return;
        if (categoryEditId) {
            await updateCategory(categoryEditId, name);
        } else {
            await createCategory(name);
        }
        closeCategoryModal();
    };

    const openSnippetModal = (snippet?: Snippet) => {
        if (snippet) {
            setSnippetEditId(snippet.id);
            setSnippetName(snippet.name);
            setSnippetContent(snippet.content);
            setSnippetCategoryId(snippet.categoryId);
        } else {
            setSnippetEditId(null);
            setSnippetName("");
            setSnippetContent("");
            setSnippetCategoryId(categories[0]?.id ?? null);
        }
        setSnippetModalOpen(true);
    };

    const closeSnippetModal = () => {
        setSnippetModalOpen(false);
        setSnippetEditId(null);
        setSnippetName("");
        setSnippetContent("");
        setSnippetCategoryId(null);
    };

    const saveSnippetFromModal = async () => {
        const name = snippetName.trim();
        const content = snippetContent.trim();
        const categoryId = snippetCategoryId;
        if (!name || !categoryId) return;
        if (snippetEditId) {
            await updateSnippet(snippetEditId, name, content, categoryId);
        } else {
            await createSnippet(name, content, categoryId);
        }
        closeSnippetModal();
    };

    const handleInsertSnippet = (content: string) => {
        const vars = getUniqueVariableNames(content);
        if (vars.length === 0) {
            setInsertSnippetText(content);
            setView("chat");
            return;
        }
        setVariablesModalContent(content);
        setVariablesModalSnippetResolve(() => (filledText: string) => {
            setInsertSnippetText(filledText);
            setView("chat");
            setVariablesModalOpen(false);
            setVariablesModalSnippetResolve(null);
        });
        setVariablesModalOpen(true);
    };

    const handleVariablesSubmit = (filledText: string) => {
        variablesModalSnippetResolve?.(filledText);
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
            <ScrollArea style={{ flex: 1, width: "100%", maxWidth: 900 }} type="scroll">
                <Stack p="lg" gap="lg">
                    <Group justify="space-between" align="center">
                        <Group>
                            <Button variant="subtle" onClick={() => setView("chat")}>
                                ← Назад
                            </Button>
                            <Title order={2}>Шаблоны промптов</Title>
                        </Group>
                    </Group>

                    {/* Категории */}
                    <Stack gap="xs">
                        <Group justify="space-between" align="center">
                            <Text size="sm" fw={500}>
                                Категории
                            </Text>
                            <Button size="xs" variant="light" onClick={() => openCategoryModal()}>
                                + Категория
                            </Button>
                        </Group>
                        <Group gap="xs" wrap="wrap">
                            {categories.map((c) => (
                                <Group key={c.id} gap="sm" wrap="nowrap">
                                    <Badge variant="light" size="lg">
                                        {c.name}
                                    </Badge>
                                    <Tooltip label="Редактировать">
                                        <ActionIcon
                                            variant="subtle"
                                            size="xs"
                                            onClick={() => openCategoryModal(c)}
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
                                            onClick={() => setDeletingCategoryId(c.id)}
                                            aria-label="Удалить"
                                        >
                                            <IconTrash size={14} stroke={1.5} />
                                        </ActionIcon>
                                    </Tooltip>
                                </Group>
                            ))}
                        </Group>
                    </Stack>

                    {/* Шаблоны */}
                    <Stack gap="xs">
                        <Group justify="space-between" align="center" wrap="wrap">
                            <Text size="sm" fw={500}>
                                Шаблоны
                            </Text>
                            <Button size="xs" variant="light" onClick={() => openSnippetModal()}>
                                + Шаблон
                            </Button>
                        </Group>
                        <Group wrap="nowrap" align="flex-end" gap="sm">
                            <Select
                                size="xs"
                                data={categorySelectData}
                                value={categoryFilter}
                                onChange={(v) => {
                                    setCategoryFilter(v ?? "all");
                                    setSnippetPage(1);
                                }}
                                style={{ minWidth: 140 }}
                            />
                            <TextInput
                                placeholder="Поиск..."
                                leftSection={<IconSearch size={16} stroke={1.5} />}
                                value={searchQuery}
                                onChange={(e) => {
                                    setSearchQuery(e.currentTarget.value);
                                    setSnippetPage(1);
                                }}
                                style={{ flex: 1, minWidth: 200 }}
                            />
                        </Group>
                        {pageSnippets.length === 0 ? (
                            <Text size="xs" c="dimmed">
                                Нет шаблонов. Добавьте категорию и шаблон.
                            </Text>
                        ) : (
                            <Stack gap="xs">
                                {pageSnippets.map((s) => (
                                    <Card key={s.id} withBorder padding="sm" className="snippet-card">
                                        <Group justify="space-between" wrap="nowrap" align="flex-start">
                                            <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
                                                <Text size="sm" fw={500}>
                                                    {s.name}
                                                </Text>
                                                <Badge size="xs" variant="outline" style={{ width: "fit-content" }}>
                                                    {getCategoryName(s.categoryId)}
                                                </Badge>
                                                <ContentPreview content={s.content} />
                                            </Stack>
                                            <Group gap="sm" wrap="nowrap">
                                                <Tooltip label="Редактировать">
                                                    <ActionIcon
                                                        variant="subtle"
                                                        size="xs"
                                                        onClick={() => openSnippetModal(s)}
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
                                                        onClick={() => setDeletingSnippetId(s.id)}
                                                        aria-label="Удалить"
                                                    >
                                                        <IconTrash size={14} stroke={1.5} />
                                                    </ActionIcon>
                                                </Tooltip>
                                                <Button
                                                    size="xs"
                                                    variant="light"
                                                    onClick={() => handleInsertSnippet(s.content)}
                                                >
                                                    Вставить в чат
                                                </Button>
                                            </Group>
                                        </Group>
                                    </Card>
                                ))}
                            </Stack>
                        )}
                        {totalSnippetPages > 1 && (
                            <Group justify="center" mt="sm">
                                <Pagination
                                    total={totalSnippetPages}
                                    value={snippetPage}
                                    onChange={setSnippetPage}
                                />
                            </Group>
                        )}
                    </Stack>
                </Stack>
            </ScrollArea>

            {/* Модалка категории */}
            <Modal
                title={categoryEditId ? "Редактировать категорию" : "Добавить категорию"}
                opened={categoryModalOpen}
                onClose={closeCategoryModal}
            >
                <Stack gap="sm">
                    <TextInput
                        label="Название"
                        placeholder="Название категории"
                        value={categoryName}
                        onChange={(e) => setCategoryName(e.currentTarget.value)}
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={closeCategoryModal}>
                            Отмена
                        </Button>
                        <Button onClick={saveCategoryFromModal} disabled={!categoryName.trim()}>
                            Сохранить
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            {/* Модалка сниппета */}
            <Modal
                title={snippetEditId ? "Редактировать шаблон" : "Добавить шаблон"}
                opened={snippetModalOpen}
                onClose={closeSnippetModal}
                size="lg"
            >
                <Stack gap="sm">
                    <TextInput
                        label="Название"
                        placeholder="Название шаблона"
                        value={snippetName}
                        onChange={(e) => setSnippetName(e.currentTarget.value)}
                    />
                    <Select
                        label="Категория"
                        data={categories.map((c) => ({ value: c.id, label: c.name }))}
                        value={snippetCategoryId}
                        onChange={setSnippetCategoryId}
                        placeholder="Выберите категорию"
                    />
                    <Textarea
                        label="Содержимое"
                        placeholder="Текст шаблона. Используйте {название} для переменных."
                        value={snippetContent}
                        onChange={(e) => setSnippetContent(e.currentTarget.value)}
                        minRows={4}
                        maxRows={15}
                        autosize
                    />
                    <Text size="xs" c="dimmed">
                        Используйте {"{название}"} для переменных.
                    </Text>
                    {snippetContent && (
                        <Stack gap={4}>
                            <Text size="xs" fw={500} c="dimmed">
                                Превью переменных:
                            </Text>
                            <VariablePreview content={snippetContent} />
                        </Stack>
                    )}
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={closeSnippetModal}>
                            Отмена
                        </Button>
                        <Button
                            onClick={saveSnippetFromModal}
                            disabled={!snippetName.trim() || !snippetCategoryId}
                        >
                            Сохранить
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            <ConfirmModal
                opened={deletingCategoryId !== null}
                onClose={() => setDeletingCategoryId(null)}
                onConfirm={() => {
                    if (deletingCategoryId) {
                        deleteCategory(deletingCategoryId);
                        setDeletingCategoryId(null);
                    }
                }}
                message="Категория будет удалена. Нельзя удалить категорию, к которой привязаны шаблоны."
            />

            <ConfirmModal
                opened={deletingSnippetId !== null}
                onClose={() => setDeletingSnippetId(null)}
                onConfirm={() => {
                    if (deletingSnippetId) {
                        deleteSnippet(deletingSnippetId);
                        setDeletingSnippetId(null);
                    }
                }}
                message="Шаблон будет удалён безвозвратно."
            />

            <VariablesModal
                content={variablesModalContent}
                opened={variablesModalOpen}
                onClose={() => {
                    setVariablesModalOpen(false);
                    setVariablesModalSnippetResolve(null);
                }}
                onSubmit={handleVariablesSubmit}
            />
        </Box>
    );
}

import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
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
import {
    IconBrain,
    IconEdit,
    IconPin,
    IconPinFilled,
    IconPlus,
    IconSearch,
    IconTrash,
    IconTrashX,
    IconX,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import type { AgentMemory, MemoryCategory } from "../types";
import { ConfirmModal } from "./ConfirmModal";
import { formatRelativeTime } from "../utils/formatDate";

const PAGE_SIZE = 20;

const CATEGORY_COLORS: Record<string, string> = {
    fact: "blue",
    decision: "violet",
    preference: "orange",
    context: "teal",
    learning: "green",
};

const CATEGORIES: MemoryCategory[] = ["fact", "decision", "preference", "context", "learning"];

export default function MemoryPage() {
    const { t } = useTranslation();
    const {
        agentMemories,
        agentMemoriesLoading,
        loadAgentMemories,
        createAgentMemory,
        updateAgentMemory,
        deleteAgentMemory,
        deleteAllAgentMemories,
    } = useChatStore();

    const [categoryFilter, setCategoryFilter] = useState<string>("all");
    const [searchQuery, setSearchQuery] = useState("");
    const [page, setPage] = useState(1);

    // Modal state
    const [modalOpen, setModalOpen] = useState(false);
    const [editId, setEditId] = useState<string | null>(null);
    const [formContent, setFormContent] = useState("");
    const [formCategory, setFormCategory] = useState<string>("fact");

    // Delete state
    const [deletingId, setDeletingId] = useState<string | null>(null);
    const [clearAllOpen, setClearAllOpen] = useState(false);

    useEffect(() => {
        loadAgentMemories();
    }, [loadAgentMemories]);

    const filteredMemories = useMemo(() => {
        let list = agentMemories;
        if (categoryFilter !== "all") {
            list = list.filter((m) => m.category === categoryFilter);
        }
        const q = searchQuery.trim().toLowerCase();
        if (q) {
            list = list.filter((m) => m.content.toLowerCase().includes(q));
        }
        return list;
    }, [agentMemories, categoryFilter, searchQuery]);

    const totalPages = Math.max(1, Math.ceil(filteredMemories.length / PAGE_SIZE));
    const pageItems = useMemo(() => {
        const start = (page - 1) * PAGE_SIZE;
        return filteredMemories.slice(start, start + PAGE_SIZE);
    }, [filteredMemories, page]);

    useEffect(() => {
        if (page > totalPages) setPage(1);
    }, [totalPages, page]);

    const openCreateModal = () => {
        setEditId(null);
        setFormContent("");
        setFormCategory("fact");
        setModalOpen(true);
    };

    const openEditModal = (memory: AgentMemory) => {
        setEditId(memory.id);
        setFormContent(memory.content);
        setFormCategory(memory.category);
        setModalOpen(true);
    };

    const handleSave = async () => {
        if (!formContent.trim()) return;
        if (editId) {
            await updateAgentMemory(editId, { content: formContent, category: formCategory });
        } else {
            await createAgentMemory(formContent, formCategory);
        }
        setModalOpen(false);
    };

    const handleDelete = async () => {
        if (deletingId) {
            await deleteAgentMemory(deletingId);
            setDeletingId(null);
        }
    };

    const handleClearAll = async () => {
        await deleteAllAgentMemories();
        setClearAllOpen(false);
    };

    const handleTogglePin = async (memory: AgentMemory) => {
        await updateAgentMemory(memory.id, { isPinned: !memory.isPinned });
    };

    const categoryOptions = [
        { value: "all", label: t("memory.categories.all") },
        ...CATEGORIES.map((c) => ({
            value: c,
            label: t(`memory.categories.${c}`),
        })),
    ];

    const formCategoryOptions = CATEGORIES.map((c) => ({
        value: c,
        label: t(`memory.categories.${c}`),
    }));

    return (
        <Box p="md" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
            {/* Header */}
            <Group justify="space-between" mb="md">
                <Group gap="sm">
                    <Title order={3}>{t("memory.title")}</Title>
                    <Badge variant="light" size="lg">
                        {t("memory.totalMemories", { count: agentMemories.length })}
                    </Badge>
                </Group>
                <Group gap="xs">
                    <Button
                        leftSection={<IconPlus size={16} stroke={1.5} />}
                        size="sm"
                        onClick={openCreateModal}
                    >
                        {t("memory.addMemory")}
                    </Button>
                    {agentMemories.length > 0 && (
                        <Button
                            leftSection={<IconTrashX size={16} stroke={1.5} />}
                            size="sm"
                            variant="light"
                            color="red"
                            onClick={() => setClearAllOpen(true)}
                        >
                            {t("memory.clearAll")}
                        </Button>
                    )}
                    <Tooltip label={t("common.close")}>
                        <ActionIcon variant="subtle" size="lg" onClick={() => useChatStore.getState().setView("chat")}>
                            <IconX size={20} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Group>
            </Group>

            {/* Filters */}
            <Group mb="md" gap="sm">
                <Select
                    data={categoryOptions}
                    value={categoryFilter}
                    onChange={(v) => {
                        setCategoryFilter(v || "all");
                        setPage(1);
                    }}
                    size="sm"
                    w={160}
                />
                <TextInput
                    placeholder={t("memory.searchPlaceholder")}
                    leftSection={<IconSearch size={16} stroke={1.5} />}
                    value={searchQuery}
                    onChange={(e) => {
                        setSearchQuery(e.currentTarget.value);
                        setPage(1);
                    }}
                    size="sm"
                    style={{ flex: 1 }}
                />
            </Group>

            {/* List */}
            <ScrollArea style={{ flex: 1 }} offsetScrollbars>
                {agentMemoriesLoading ? (
                    <Text c="dimmed" ta="center" py="xl">
                        {t("common.loading")}...
                    </Text>
                ) : filteredMemories.length === 0 ? (
                    <Stack align="center" py="xl" gap="sm">
                        <IconBrain size={48} stroke={1} style={{ opacity: 0.3 }} />
                        <Text c="dimmed" ta="center" maw={400}>
                            {searchQuery || categoryFilter !== "all"
                                ? t("common.noResults")
                                : t("memory.emptyState")}
                        </Text>
                    </Stack>
                ) : (
                    <Stack gap="xs">
                        {pageItems.map((memory) => (
                            <Card key={memory.id} withBorder padding="sm">
                                <Group justify="space-between" wrap="nowrap" align="flex-start">
                                    <Stack gap={4} style={{ flex: 1, minWidth: 0 }}>
                                        <Text size="sm">{memory.content}</Text>
                                        <Group gap="xs">
                                            <Badge
                                                variant="light"
                                                color={CATEGORY_COLORS[memory.category] || "gray"}
                                                size="sm"
                                            >
                                                {t(`memory.categories.${memory.category}`)}
                                            </Badge>
                                            <Text size="xs" c="dimmed">
                                                {formatRelativeTime(memory.updatedAt)}
                                            </Text>
                                        </Group>
                                    </Stack>
                                    <Group gap={4} wrap="nowrap">
                                        <Tooltip
                                            label={
                                                memory.isPinned
                                                    ? t("memory.unpinMemory")
                                                    : t("memory.pinMemory")
                                            }
                                        >
                                            <ActionIcon
                                                variant="subtle"
                                                size="sm"
                                                onClick={() => handleTogglePin(memory)}
                                            >
                                                {memory.isPinned ? (
                                                    <IconPinFilled size={14} stroke={1.5} />
                                                ) : (
                                                    <IconPin size={14} stroke={1.5} />
                                                )}
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("memory.editMemory")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="sm"
                                                onClick={() => openEditModal(memory)}
                                            >
                                                <IconEdit size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("memory.deleteMemory")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="sm"
                                                color="red"
                                                onClick={() => setDeletingId(memory.id)}
                                            >
                                                <IconTrash size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                </Group>
                            </Card>
                        ))}
                    </Stack>
                )}
            </ScrollArea>

            {totalPages > 1 && (
                <Group justify="center" mt="sm">
                    <Pagination total={totalPages} value={page} onChange={setPage} size="sm" />
                </Group>
            )}

            {/* Add/Edit Modal */}
            <Modal
                opened={modalOpen}
                onClose={() => setModalOpen(false)}
                title={editId ? t("memory.editMemory") : t("memory.addMemory")}
                size="md"
            >
                <Stack gap="sm">
                    <Textarea
                        label={t("memory.contentLabel")}
                        value={formContent}
                        onChange={(e) => setFormContent(e.currentTarget.value)}
                        minRows={3}
                        autosize
                    />
                    <Select
                        label={t("memory.categoryLabel")}
                        data={formCategoryOptions}
                        value={formCategory}
                        onChange={(v) => setFormCategory(v || "fact")}
                    />
                    <Group justify="flex-end" gap="xs">
                        <Button variant="default" onClick={() => setModalOpen(false)}>
                            {t("memory.cancel")}
                        </Button>
                        <Button onClick={handleSave} disabled={!formContent.trim()}>
                            {t("memory.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            {/* Delete confirmation */}
            <ConfirmModal
                opened={deletingId !== null}
                onClose={() => setDeletingId(null)}
                onConfirm={handleDelete}
                message={t("memory.deleteConfirm")}
            />

            {/* Clear all confirmation */}
            <ConfirmModal
                opened={clearAllOpen}
                onClose={() => setClearAllOpen(false)}
                onConfirm={handleClearAll}
                message={t("memory.clearConfirm")}
            />
        </Box>
    );
}

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
    IconCopy,
    IconLock,
    IconPencil,
    IconPlus,
    IconSearch,
    IconTrash,
} from "@tabler/icons-react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useChatStore } from "../store/chatStore";
import { PROMPT_CATEGORIES } from "../constants/promptCategories";
import { notify } from "../utils/notify";
import { ConfirmModal } from "./ConfirmModal";
import { getUniqueVariableNames, fillVariables } from "./VariablesModal";
import type { PromptLibraryItem } from "../types";

const PAGE_SIZE = 20;
const PREVIEW_MAX = 120;
const VARIABLE_REGEX = /\{([^}]+)\}/g;

function ContentPreview({ content }: { content: string }) {
    const truncated = content.length > PREVIEW_MAX ? content.slice(0, PREVIEW_MAX) + "…" : content;
    const parts: { text: string; isVar: boolean }[] = [];
    let lastIndex = 0;
    let m: RegExpExecArray | null;
    const re = new RegExp(VARIABLE_REGEX.source, "g");
    while ((m = re.exec(truncated)) !== null) {
        if (m.index > lastIndex) parts.push({ text: truncated.slice(lastIndex, m.index), isVar: false });
        parts.push({ text: m[0], isVar: true });
        lastIndex = m.index + m[0].length;
    }
    if (lastIndex < truncated.length) parts.push({ text: truncated.slice(lastIndex), isVar: false });
    if (parts.length === 0) parts.push({ text: truncated, isVar: false });

    return (
        <Text size="xs" c="dimmed" lineClamp={2}>
            {parts.map((p, i) =>
                p.isVar ? (
                    <Text key={i} span c="blue" inherit>{p.text}</Text>
                ) : (
                    <span key={i}>{p.text}</span>
                )
            )}
        </Text>
    );
}

export function PromptLibraryPage() {
    const { t } = useTranslation();
    const {
        promptLibrary,
        fetchPromptLibrary,
        createPromptLibraryItem,
        updatePromptLibraryItem,
        deletePromptLibraryItem,
        setView,
    } = useChatStore();

    const [categoryFilter, setCategoryFilter] = useState<string>("all");
    const [searchQuery, setSearchQuery] = useState("");
    const [page, setPage] = useState(1);

    const [editModalOpen, setEditModalOpen] = useState(false);
    const [editId, setEditId] = useState<string | null>(null);
    const [formTitle, setFormTitle] = useState("");
    const [formDescription, setFormDescription] = useState("");
    const [formContent, setFormContent] = useState("");
    const [formCategory, setFormCategory] = useState<string>("general");
    const [titleError, setTitleError] = useState<string | null>(null);
    const [contentError, setContentError] = useState<string | null>(null);

    const [deletingId, setDeletingId] = useState<string | null>(null);

    const [varsModalOpen, setVarsModalOpen] = useState(false);
    const [varsContent, setVarsContent] = useState("");
    const [varsValues, setVarsValues] = useState<Record<string, string>>({});

    useEffect(() => {
        fetchPromptLibrary(
            categoryFilter === "all" ? undefined : categoryFilter,
            searchQuery || undefined,
        );
    }, [categoryFilter, searchQuery, fetchPromptLibrary]);

    const categorySelectData = useMemo(
        () => [
            { value: "all", label: t("promptLibrary.allCategories") },
            ...PROMPT_CATEGORIES.map((c) => ({
                value: c.id,
                label: t(`promptLibrary.categories.${c.id}`),
            })),
        ],
        [t]
    );

    const totalPages = Math.max(1, Math.ceil(promptLibrary.length / PAGE_SIZE));
    const pageItems = useMemo(() => {
        const start = (page - 1) * PAGE_SIZE;
        return promptLibrary.slice(start, start + PAGE_SIZE);
    }, [promptLibrary, page]);

    const getCategoryLabel = (id: string) => t(`promptLibrary.categories.${id}`, id);

    const openEditModal = (item?: PromptLibraryItem) => {
        if (item) {
            setEditId(item.id);
            setFormTitle(item.title);
            setFormDescription(item.description);
            setFormContent(item.content);
            setFormCategory(item.category);
        } else {
            setEditId(null);
            setFormTitle("");
            setFormDescription("");
            setFormContent("");
            setFormCategory("general");
        }
        setTitleError(null);
        setContentError(null);
        setEditModalOpen(true);
    };

    const closeEditModal = () => {
        setEditModalOpen(false);
        setEditId(null);
    };

    const saveFromModal = async () => {
        let hasError = false;
        if (!formTitle.trim()) { setTitleError(t("common.fieldRequired")); hasError = true; } else { setTitleError(null); }
        if (!formContent.trim()) { setContentError(t("common.fieldRequired")); hasError = true; } else { setContentError(null); }
        if (hasError) return;

        const data = {
            title: formTitle.trim(),
            description: formDescription.trim(),
            content: formContent.trim(),
            category: formCategory,
        };

        if (editId) {
            await updatePromptLibraryItem(editId, data);
        } else {
            await createPromptLibraryItem(data);
        }
        closeEditModal();
    };

    const handleCopy = async (item: PromptLibraryItem) => {
        const vars = getUniqueVariableNames(item.content);
        if (vars.length === 0) {
            await writeText(item.content);
            notify.success(t("promptLibrary.copied"));
            return;
        }
        setVarsContent(item.content);
        setVarsValues({});
        setVarsModalOpen(true);
    };

    const handleVarsSubmit = async () => {
        const filled = fillVariables(varsContent, varsValues);
        await writeText(filled);
        notify.success(t("promptLibrary.copied"));
        setVarsModalOpen(false);
        setVarsValues({});
    };

    const varsNames = useMemo(() => getUniqueVariableNames(varsContent), [varsContent]);

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
                                ← {t("common.back")}
                            </Button>
                            <Title order={2}>{t("promptLibrary.title")}</Title>
                        </Group>
                        <Button
                            size="xs"
                            variant="light"
                            leftSection={<IconPlus size={14} stroke={1.5} />}
                            onClick={() => openEditModal()}
                        >
                            {t("promptLibrary.addPrompt")}
                        </Button>
                    </Group>

                    <Group wrap="nowrap" align="flex-end" gap="sm">
                        <Select
                            size="xs"
                            data={categorySelectData}
                            value={categoryFilter}
                            onChange={(v) => {
                                setCategoryFilter(v ?? "all");
                                setPage(1);
                            }}
                            style={{ minWidth: 140 }}
                        />
                        <TextInput
                            placeholder={t("promptLibrary.search")}
                            leftSection={<IconSearch size={16} stroke={1.5} />}
                            value={searchQuery}
                            onChange={(e) => {
                                setSearchQuery(e.currentTarget.value);
                                setPage(1);
                            }}
                            style={{ flex: 1, minWidth: 200 }}
                        />
                    </Group>

                    {pageItems.length === 0 ? (
                        <Text size="sm" c="dimmed" ta="center" py="xl">
                            {t("promptLibrary.noPrompts")}
                        </Text>
                    ) : (
                        <Stack gap="xs">
                            {pageItems.map((item) => (
                                <Card key={item.id} withBorder padding="sm">
                                    <Group justify="space-between" wrap="nowrap" align="flex-start">
                                        <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
                                            <Group gap="xs" wrap="nowrap">
                                                <Text size="sm" fw={500} truncate>
                                                    {item.title}
                                                </Text>
                                                {item.isBuiltin && (
                                                    <Badge size="xs" variant="outline" color="gray">
                                                        {t("promptLibrary.builtinBadge")}
                                                    </Badge>
                                                )}
                                            </Group>
                                            {item.description && (
                                                <Text size="xs" c="dimmed" lineClamp={1}>
                                                    {item.description}
                                                </Text>
                                            )}
                                            <Badge size="xs" variant="outline" style={{ width: "fit-content" }}>
                                                {getCategoryLabel(item.category)}
                                            </Badge>
                                            <ContentPreview content={item.content} />
                                        </Stack>
                                        <Group gap="sm" wrap="nowrap">
                                            {!item.isBuiltin ? (
                                                <>
                                                    <Tooltip label={t("common.edit")}>
                                                        <ActionIcon
                                                            variant="subtle"
                                                            size="xs"
                                                            onClick={() => openEditModal(item)}
                                                        >
                                                            <IconPencil size={14} stroke={1.5} />
                                                        </ActionIcon>
                                                    </Tooltip>
                                                    <Tooltip label={t("common.delete")}>
                                                        <ActionIcon
                                                            variant="subtle"
                                                            size="xs"
                                                            color="red"
                                                            onClick={() => setDeletingId(item.id)}
                                                        >
                                                            <IconTrash size={14} stroke={1.5} />
                                                        </ActionIcon>
                                                    </Tooltip>
                                                </>
                                            ) : (
                                                <Tooltip label={t("promptLibrary.builtinBadge")}>
                                                    <ActionIcon variant="subtle" size="xs" disabled>
                                                        <IconLock size={14} stroke={1.5} />
                                                    </ActionIcon>
                                                </Tooltip>
                                            )}
                                            <Button
                                                size="xs"
                                                variant="light"
                                                leftSection={<IconCopy size={14} stroke={1.5} />}
                                                onClick={() => handleCopy(item)}
                                            >
                                                {t("promptLibrary.copyPrompt")}
                                            </Button>
                                        </Group>
                                    </Group>
                                </Card>
                            ))}
                        </Stack>
                    )}

                    {totalPages > 1 && (
                        <Group justify="center" mt="sm">
                            <Pagination total={totalPages} value={page} onChange={setPage} />
                        </Group>
                    )}
                </Stack>
            </ScrollArea>

            {/* Create/Edit Modal */}
            <Modal
                title={editId ? t("promptLibrary.editPrompt") : t("promptLibrary.addPrompt")}
                opened={editModalOpen}
                onClose={closeEditModal}
                size="md"
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("promptLibrary.promptTitle")}
                        value={formTitle}
                        onChange={(e) => { setFormTitle(e.currentTarget.value); setTitleError(null); }}
                        error={titleError}
                    />
                    <TextInput
                        label={t("promptLibrary.promptDescription")}
                        value={formDescription}
                        onChange={(e) => setFormDescription(e.currentTarget.value)}
                    />
                    <Select
                        label={t("promptLibrary.promptCategory")}
                        data={PROMPT_CATEGORIES.map((c) => ({
                            value: c.id,
                            label: t(`promptLibrary.categories.${c.id}`),
                        }))}
                        value={formCategory}
                        onChange={(v) => setFormCategory(v ?? "general")}
                    />
                    <Textarea
                        label={t("promptLibrary.promptContent")}
                        value={formContent}
                        onChange={(e) => { setFormContent(e.currentTarget.value); setContentError(null); }}
                        error={contentError}
                        minRows={6}
                        maxRows={15}
                        autosize
                        description={t("promptLibrary.variablesHint")}
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={closeEditModal}>
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={saveFromModal}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            {/* Delete Confirm */}
            <ConfirmModal
                opened={deletingId !== null}
                onClose={() => setDeletingId(null)}
                onConfirm={() => {
                    if (deletingId) {
                        deletePromptLibraryItem(deletingId);
                        setDeletingId(null);
                    }
                }}
                message={t("promptLibrary.deleteConfirm")}
            />

            {/* Variables Modal */}
            <Modal
                title={t("promptLibrary.fillVariables")}
                size="md"
                opened={varsModalOpen}
                onClose={() => setVarsModalOpen(false)}
            >
                <Stack gap="sm">
                    {varsNames.map((name) => (
                        <TextInput
                            key={name}
                            label={name}
                            value={varsValues[name] ?? ""}
                            onChange={(e) =>
                                setVarsValues((prev) => ({ ...prev, [name]: e.currentTarget.value }))
                            }
                            placeholder={name}
                        />
                    ))}
                    <Group justify="flex-end" gap="sm" mt="md">
                        <Button variant="subtle" onClick={() => setVarsModalOpen(false)}>
                            {t("common.cancel")}
                        </Button>
                        <Button
                            leftSection={<IconCopy size={14} stroke={1.5} />}
                            onClick={handleVarsSubmit}
                        >
                            {t("promptLibrary.copyPrompt")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>
        </Box>
    );
}

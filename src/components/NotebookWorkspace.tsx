import { useEffect, useState, useCallback } from "react";
import {
    Box,
    Text,
    Button,
    Group,
    Badge,
    Table,
    ActionIcon,
    Menu,
    Modal,
    TextInput,
    Textarea,
    Stack,
    SimpleGrid,
    Card,
    Loader,
    UnstyledButton,
} from "@mantine/core";
import {
    IconArrowLeft,
    IconPlus,
    IconTrash,
    IconFile,
    IconWorld,
    IconBrandYoutube,
    IconFileText,
    IconFileDescription,
    IconHeadphones,
    IconTopologyStar3 as IconTopology,
    IconCards,
    IconUpload,
} from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useChatStore } from "../store/chatStore";
import type { KbDocument } from "../types";

function sourceIcon(sourceType: string) {
    switch (sourceType) {
        case "url":
            return <IconWorld size={16} stroke={1.5} />;
        case "youtube":
            return <IconBrandYoutube size={16} stroke={1.5} />;
        case "text":
            return <IconFileText size={16} stroke={1.5} />;
        default:
            return <IconFile size={16} stroke={1.5} />;
    }
}

function statusBadge(status: string, t: (key: string) => string) {
    const colorMap: Record<string, string> = {
        pending: "yellow",
        indexing: "blue",
        indexed: "green",
        failed: "red",
    };
    const labelMap: Record<string, string> = {
        pending: t("notebook.pending"),
        indexing: t("notebook.indexing"),
        indexed: t("notebook.indexed"),
        failed: t("notebook.failed"),
    };
    return (
        <Badge size="xs" variant="light" color={colorMap[status] ?? "gray"}>
            {labelMap[status] ?? status}
        </Badge>
    );
}

function formatSize(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

interface IndexingProgressPayload {
    kbId: string;
    documentId: string;
    status: string;
    totalChunks?: number;
    error?: string;
}

export default function NotebookWorkspace() {
    const { t } = useTranslation();
    const activeNotebookId = useChatStore((s) => s.activeNotebookId);
    const notebooks = useChatStore((s) => s.notebooks);
    const notebookDocuments = useChatStore((s) => s.notebookDocuments);
    const notebookLoading = useChatStore((s) => s.notebookLoading);
    const setActiveNotebook = useChatStore((s) => s.setActiveNotebook);
    const loadNotebookDocuments = useChatStore((s) => s.loadNotebookDocuments);
    const addNotebookSource = useChatStore((s) => s.addNotebookSource);
    const removeNotebookDocument = useChatStore((s) => s.removeNotebookDocument);
    const loadNotebooks = useChatStore((s) => s.loadNotebooks);

    const notebook = notebooks.find((n) => n.id === activeNotebookId);

    const [urlModalOpen, setUrlModalOpen] = useState(false);
    const [urlValue, setUrlValue] = useState("");
    const [ytModalOpen, setYtModalOpen] = useState(false);
    const [ytValue, setYtValue] = useState("");
    const [textModalOpen, setTextModalOpen] = useState(false);
    const [textName, setTextName] = useState("");
    const [textContent, setTextContent] = useState("");
    const [dragOver, setDragOver] = useState(false);

    useEffect(() => {
        if (activeNotebookId) {
            loadNotebookDocuments(activeNotebookId);
        }
    }, [activeNotebookId]);

    // Listen for indexing progress events to refresh doc list
    useEffect(() => {
        if (!notebook?.kbId) return;
        const kbId = notebook.kbId;
        const unlisten = listen<IndexingProgressPayload>("kb-indexing-progress", (event) => {
            if (event.payload.kbId === kbId) {
                if (event.payload.status === "done" || event.payload.status === "failed") {
                    if (activeNotebookId) {
                        loadNotebookDocuments(activeNotebookId);
                        loadNotebooks();
                    }
                }
            }
        });
        return () => {
            unlisten.then((fn) => fn());
        };
    }, [notebook?.kbId, activeNotebookId]);

    const handleAddFiles = useCallback(async () => {
        const selected = await open({
            multiple: true,
            filters: [
                {
                    name: "Documents",
                    extensions: ["pdf", "txt", "md", "html", "htm", "docx", "csv", "json"],
                },
            ],
        });
        if (selected && activeNotebookId) {
            const paths = Array.isArray(selected) ? selected : [selected];
            await addNotebookSource(activeNotebookId, "file", { filePaths: paths });
        }
    }, [activeNotebookId, addNotebookSource]);

    const handleAddUrl = async () => {
        if (!urlValue.trim() || !activeNotebookId) return;
        await addNotebookSource(activeNotebookId, "url", { url: urlValue.trim() });
        setUrlModalOpen(false);
        setUrlValue("");
    };

    const handleAddYoutube = async () => {
        if (!ytValue.trim() || !activeNotebookId) return;
        await addNotebookSource(activeNotebookId, "youtube", { url: ytValue.trim() });
        setYtModalOpen(false);
        setYtValue("");
    };

    const handleAddText = async () => {
        if (!textName.trim() || !textContent.trim() || !activeNotebookId) return;
        await addNotebookSource(activeNotebookId, "text", {
            name: textName.trim(),
            content: textContent.trim(),
        });
        setTextModalOpen(false);
        setTextName("");
        setTextContent("");
    };

    const handleRemove = async (doc: KbDocument) => {
        if (!activeNotebookId) return;
        await removeNotebookDocument(activeNotebookId, doc.id);
    };

    // Drag & drop handlers
    const handleDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(true);
    };
    const handleDragLeave = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(false);
    };
    const handleDrop = async (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(false);
        if (!activeNotebookId) return;
        const files = Array.from(e.dataTransfer.files);
        const paths = files.map((f) => f.name);
        // Note: Web drag & drop doesn't give full paths in Tauri 2.
        // For now, file drag & drop uses the file dialog as primary method.
        if (paths.length > 0) {
            // Use file dialog instead since we can't get full paths from drag events
            handleAddFiles();
        }
    };

    if (!notebook) {
        return (
            <Box p="xl">
                <Text c="dimmed">{t("notebook.notFound")}</Text>
            </Box>
        );
    }

    const tools = [
        { icon: IconFileDescription, label: "Summary" },
        { icon: IconHeadphones, label: "Audio Overview" },
        { icon: IconTopology, label: "Mind Map" },
        { icon: IconCards, label: "Flashcards" },
    ];

    return (
        <Box
            p="xl"
            style={{ height: "100%", overflow: "auto" }}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
        >
            {/* Header */}
            <Group mb="lg">
                <UnstyledButton onClick={() => setActiveNotebook(null)}>
                    <Group gap={4}>
                        <IconArrowLeft size={16} stroke={1.5} />
                        <Text size="sm" c="dimmed">
                            {t("notebook.backToList")}
                        </Text>
                    </Group>
                </UnstyledButton>
            </Group>

            <Group justify="space-between" mb="md">
                <Group gap="sm">
                    <Text size="xl" fw={700}>
                        {notebook.name}
                    </Text>
                    <Badge variant="light" color="violet" size="sm">
                        {t("notebook.documents", { count: notebook.documentCount })}
                    </Badge>
                    <Badge variant="light" color="gray" size="sm">
                        {t("notebook.chunks", { count: notebook.totalChunks })}
                    </Badge>
                </Group>
                <Menu position="bottom-end">
                    <Menu.Target>
                        <Button leftSection={<IconPlus size={16} />} color="violet" size="sm">
                            {t("notebook.addDocuments")}
                        </Button>
                    </Menu.Target>
                    <Menu.Dropdown>
                        <Menu.Item
                            leftSection={<IconFile size={16} stroke={1.5} />}
                            onClick={handleAddFiles}
                        >
                            {t("notebook.addFile")}
                        </Menu.Item>
                        <Menu.Item
                            leftSection={<IconWorld size={16} stroke={1.5} />}
                            onClick={() => setUrlModalOpen(true)}
                        >
                            {t("notebook.addUrl")}
                        </Menu.Item>
                        <Menu.Item
                            leftSection={<IconBrandYoutube size={16} stroke={1.5} />}
                            onClick={() => setYtModalOpen(true)}
                        >
                            {t("notebook.addYoutube")}
                        </Menu.Item>
                        <Menu.Item
                            leftSection={<IconFileText size={16} stroke={1.5} />}
                            onClick={() => setTextModalOpen(true)}
                        >
                            {t("notebook.addText")}
                        </Menu.Item>
                    </Menu.Dropdown>
                </Menu>
            </Group>

            {notebook.description && (
                <Text size="sm" c="dimmed" mb="lg">
                    {notebook.description}
                </Text>
            )}

            {/* Drag overlay */}
            {dragOver && (
                <Box
                    style={{
                        position: "fixed",
                        inset: 0,
                        zIndex: 100,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        background: "rgba(0,0,0,0.3)",
                        border: "3px dashed var(--mantine-color-violet-5)",
                        borderRadius: 12,
                        pointerEvents: "none",
                    }}
                >
                    <Group gap="sm">
                        <IconUpload size={32} color="var(--mantine-color-violet-5)" />
                        <Text size="lg" fw={600} c="white">
                            {t("notebook.dropDocuments")}
                        </Text>
                    </Group>
                </Box>
            )}

            {/* Documents list */}
            {notebookDocuments.length === 0 && !notebookLoading ? (
                <Box
                    style={{
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "center",
                        justifyContent: "center",
                        padding: 40,
                        gap: 12,
                    }}
                >
                    <IconFile size={48} stroke={1} color="var(--mantine-color-dimmed)" />
                    <Text c="dimmed">{t("notebook.noDocuments")}</Text>
                    <Text size="xs" c="dimmed">
                        {t("notebook.supportedFormats")}
                    </Text>
                </Box>
            ) : (
                <Table striped highlightOnHover mb="xl">
                    <Table.Thead>
                        <Table.Tr>
                            <Table.Th style={{ width: 32 }}></Table.Th>
                            <Table.Th>{t("notebook.name")}</Table.Th>
                            <Table.Th style={{ width: 100 }}>Size</Table.Th>
                            <Table.Th style={{ width: 100 }}>Status</Table.Th>
                            <Table.Th style={{ width: 80 }}>Chunks</Table.Th>
                            <Table.Th style={{ width: 40 }}></Table.Th>
                        </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                        {notebookDocuments.map((doc) => (
                            <Table.Tr key={doc.id}>
                                <Table.Td>{sourceIcon(doc.sourceType)}</Table.Td>
                                <Table.Td>
                                    <Text size="sm" lineClamp={1}>
                                        {doc.name}
                                    </Text>
                                    {doc.sourceUrl && (
                                        <Text size="xs" c="dimmed" lineClamp={1}>
                                            {doc.sourceUrl}
                                        </Text>
                                    )}
                                </Table.Td>
                                <Table.Td>
                                    <Text size="xs" c="dimmed">
                                        {formatSize(doc.fileSize)}
                                    </Text>
                                </Table.Td>
                                <Table.Td>{statusBadge(doc.indexingStatus, t)}</Table.Td>
                                <Table.Td>
                                    <Text size="xs" c="dimmed">
                                        {doc.chunkCount}
                                    </Text>
                                </Table.Td>
                                <Table.Td>
                                    <ActionIcon
                                        variant="subtle"
                                        color="red"
                                        size="sm"
                                        onClick={() => handleRemove(doc)}
                                    >
                                        <IconTrash size={14} />
                                    </ActionIcon>
                                </Table.Td>
                            </Table.Tr>
                        ))}
                    </Table.Tbody>
                </Table>
            )}

            {notebookLoading && (
                <Group justify="center" py="md">
                    <Loader size="sm" color="violet" />
                    <Text size="sm" c="dimmed">
                        {t("notebook.fetching")}
                    </Text>
                </Group>
            )}

            {/* Tools teaser */}
            <Text size="lg" fw={600} mb="sm" mt="xl">
                {t("notebook.tools")}
            </Text>
            <SimpleGrid cols={{ base: 2, sm: 4 }} spacing="sm">
                {tools.map((tool) => (
                    <Card
                        key={tool.label}
                        withBorder
                        p="md"
                        radius="md"
                        style={{ opacity: 0.5, cursor: "not-allowed" }}
                    >
                        <Group gap="sm">
                            <tool.icon size={20} stroke={1.5} color="var(--mantine-color-dimmed)" />
                            <Box>
                                <Text size="sm" fw={500}>
                                    {tool.label}
                                </Text>
                                <Text size="xs" c="dimmed">
                                    {t("notebook.comingSoon")}
                                </Text>
                            </Box>
                        </Group>
                    </Card>
                ))}
            </SimpleGrid>

            {/* URL modal */}
            <Modal
                opened={urlModalOpen}
                onClose={() => setUrlModalOpen(false)}
                title={t("notebook.addUrl")}
                centered
            >
                <Stack gap="sm">
                    <TextInput
                        label="URL"
                        placeholder={t("notebook.urlPlaceholder")}
                        value={urlValue}
                        onChange={(e) => setUrlValue(e.currentTarget.value)}
                        autoFocus
                        onKeyDown={(e) => e.key === "Enter" && handleAddUrl()}
                    />
                    <Button
                        color="violet"
                        onClick={handleAddUrl}
                        disabled={!urlValue.trim()}
                        loading={notebookLoading}
                    >
                        {t("notebook.addDocuments")}
                    </Button>
                </Stack>
            </Modal>

            {/* YouTube modal */}
            <Modal
                opened={ytModalOpen}
                onClose={() => setYtModalOpen(false)}
                title={t("notebook.addYoutube")}
                centered
            >
                <Stack gap="sm">
                    <TextInput
                        label="YouTube URL"
                        placeholder={t("notebook.youtubePlaceholder")}
                        value={ytValue}
                        onChange={(e) => setYtValue(e.currentTarget.value)}
                        autoFocus
                        onKeyDown={(e) => e.key === "Enter" && handleAddYoutube()}
                    />
                    <Button
                        color="violet"
                        onClick={handleAddYoutube}
                        disabled={!ytValue.trim()}
                        loading={notebookLoading}
                    >
                        {t("notebook.addDocuments")}
                    </Button>
                </Stack>
            </Modal>

            {/* Text modal */}
            <Modal
                opened={textModalOpen}
                onClose={() => setTextModalOpen(false)}
                title={t("notebook.addText")}
                centered
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("notebook.textName")}
                        placeholder={t("notebook.namePlaceholder")}
                        value={textName}
                        onChange={(e) => setTextName(e.currentTarget.value)}
                        autoFocus
                    />
                    <Textarea
                        label={t("notebook.textContent")}
                        placeholder={t("notebook.descriptionPlaceholder")}
                        value={textContent}
                        onChange={(e) => setTextContent(e.currentTarget.value)}
                        minRows={5}
                    />
                    <Button
                        color="violet"
                        onClick={handleAddText}
                        disabled={!textName.trim() || !textContent.trim()}
                        loading={notebookLoading}
                    >
                        {t("notebook.addDocuments")}
                    </Button>
                </Stack>
            </Modal>
        </Box>
    );
}

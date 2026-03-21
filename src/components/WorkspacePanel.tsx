import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Collapse,
    Group,
    Modal,
    ScrollArea,
    Stack,
    Text,
    TextInput,
    Textarea,
    Select,
    Tooltip,
} from "@mantine/core";
import {
    IconChevronDown,
    IconChevronRight,
    IconCode,
    IconCopy,
    IconFileText,
    IconPackage,
    IconPlus,
    IconDatabase,
    IconTrash,
    IconX,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";

const TYPE_COLORS: Record<string, string> = {
    code: "blue",
    text: "gray",
    data: "teal",
    image: "orange",
};

const TYPE_ICONS: Record<string, React.ReactNode> = {
    code: <IconCode size={14} stroke={1.5} />,
    text: <IconFileText size={14} stroke={1.5} />,
    data: <IconDatabase size={14} stroke={1.5} />,
};

export function WorkspacePanel() {
    const { t } = useTranslation();
    const workspaceArtifacts = useChatStore((s) => s.workspaceArtifacts);
    const setShowWorkspacePanel = useChatStore((s) => s.setShowWorkspacePanel);
    const activeChatId = useChatStore((s) => s.activeChatId);
    const loadWorkspaceArtifacts = useChatStore((s) => s.loadWorkspaceArtifacts);
    const createWorkspaceArtifact = useChatStore((s) => s.createWorkspaceArtifact);
    const deleteWorkspaceArtifact = useChatStore((s) => s.deleteWorkspaceArtifact);

    // Reload artifacts when panel mounts or chat changes
    useEffect(() => {
        if (activeChatId) {
            loadWorkspaceArtifacts(activeChatId);
        }
    }, [activeChatId, loadWorkspaceArtifacts]);

    const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());
    const [addModalOpen, setAddModalOpen] = useState(false);
    const [newName, setNewName] = useState("");
    const [newType, setNewType] = useState<string>("text");
    const [newContent, setNewContent] = useState("");
    const [deleteId, setDeleteId] = useState<string | null>(null);

    const toggleExpand = (id: string) => {
        setExpandedIds((prev) => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            return next;
        });
    };

    const handleAdd = async () => {
        if (!activeChatId || !newName.trim()) return;
        await createWorkspaceArtifact(activeChatId, newName.trim(), newType, newContent);
        setNewName("");
        setNewContent("");
        setNewType("text");
        setAddModalOpen(false);
    };

    const handleDelete = async () => {
        if (deleteId) {
            await deleteWorkspaceArtifact(deleteId);
            setDeleteId(null);
        }
    };

    const handleCopy = (content: string) => {
        void navigator.clipboard.writeText(content);
    };

    return (
        <Box
            style={{
                width: 300,
                borderLeft: "1px solid var(--mantine-color-default-border)",
                display: "flex",
                flexDirection: "column",
                height: "100%",
                overflow: "hidden",
            }}
        >
            {/* Header */}
            <Group gap="xs" p="xs" wrap="nowrap" style={{ borderBottom: "1px solid var(--mantine-color-default-border)" }}>
                <IconPackage size={18} stroke={1.5} />
                <Text size="sm" fw={600} style={{ flex: 1 }}>
                    {t("workspace.title")}
                </Text>
                <Tooltip label={t("workspace.addArtifact")}>
                    <ActionIcon size="sm" variant="subtle" onClick={() => setAddModalOpen(true)}>
                        <IconPlus size={16} stroke={1.5} />
                    </ActionIcon>
                </Tooltip>
                <ActionIcon size="sm" variant="subtle" onClick={() => setShowWorkspacePanel(false)}>
                    <IconX size={16} stroke={1.5} />
                </ActionIcon>
            </Group>

            {/* Content */}
            <ScrollArea style={{ flex: 1 }} p="xs">
                {workspaceArtifacts.length === 0 ? (
                    <Stack align="center" gap="xs" mt="xl">
                        <IconPackage size={40} stroke={1} style={{ opacity: 0.3 }} />
                        <Text size="sm" c="dimmed" ta="center">
                            {t("workspace.empty")}
                        </Text>
                        <Text size="xs" c="dimmed" ta="center">
                            {t("workspace.emptyDescription")}
                        </Text>
                    </Stack>
                ) : (
                    <Stack gap="xs">
                        {workspaceArtifacts.map((artifact) => {
                            const expanded = expandedIds.has(artifact.id);
                            const size = artifact.content?.length ?? 0;
                            return (
                                <Box
                                    key={artifact.id}
                                    style={{
                                        border: "1px solid var(--mantine-color-default-border)",
                                        borderRadius: "var(--mantine-radius-sm)",
                                        overflow: "hidden",
                                    }}
                                >
                                    <Group
                                        gap={6}
                                        p={6}
                                        wrap="nowrap"
                                        style={{ cursor: "pointer" }}
                                        onClick={() => toggleExpand(artifact.id)}
                                    >
                                        {expanded ? (
                                            <IconChevronDown size={14} stroke={1.5} />
                                        ) : (
                                            <IconChevronRight size={14} stroke={1.5} />
                                        )}
                                        {TYPE_ICONS[artifact.contentType] || <IconFileText size={14} stroke={1.5} />}
                                        <Text size="xs" fw={500} style={{ flex: 1, minWidth: 0 }} lineClamp={1}>
                                            {artifact.name}
                                        </Text>
                                        <Badge size="xs" color={TYPE_COLORS[artifact.contentType] || "gray"} variant="light">
                                            {artifact.contentType}
                                        </Badge>
                                    </Group>
                                    <Collapse in={expanded}>
                                        <Box p={6} pt={0}>
                                            <Text size="xs" c="dimmed" mb={4}>
                                                {size} {t("workspace.bytes")}
                                            </Text>
                                            {artifact.content && (
                                                <Box
                                                    style={{
                                                        background: "var(--mantine-color-dark-7)",
                                                        borderRadius: "var(--mantine-radius-xs)",
                                                        padding: 6,
                                                        maxHeight: 200,
                                                        overflow: "auto",
                                                        fontSize: 11,
                                                        fontFamily: "monospace",
                                                        whiteSpace: "pre-wrap",
                                                        wordBreak: "break-all",
                                                    }}
                                                >
                                                    {artifact.content}
                                                </Box>
                                            )}
                                            <Group gap={4} mt={4}>
                                                {artifact.content && (
                                                    <Tooltip label={t("workspace.copyContent")}>
                                                        <ActionIcon size="xs" variant="subtle" onClick={() => handleCopy(artifact.content!)}>
                                                            <IconCopy size={12} stroke={1.5} />
                                                        </ActionIcon>
                                                    </Tooltip>
                                                )}
                                                <Tooltip label={t("workspace.deleteArtifact")}>
                                                    <ActionIcon size="xs" variant="subtle" color="red" onClick={() => setDeleteId(artifact.id)}>
                                                        <IconTrash size={12} stroke={1.5} />
                                                    </ActionIcon>
                                                </Tooltip>
                                            </Group>
                                        </Box>
                                    </Collapse>
                                </Box>
                            );
                        })}
                    </Stack>
                )}
            </ScrollArea>

            {/* Add Artifact Modal */}
            <Modal opened={addModalOpen} onClose={() => setAddModalOpen(false)} title={t("workspace.addArtifact")} size="md">
                <Stack gap="sm">
                    <TextInput
                        label={t("workspace.name")}
                        value={newName}
                        onChange={(e) => setNewName(e.currentTarget.value)}
                        placeholder="report.md"
                    />
                    <Select
                        label={t("workspace.contentType")}
                        value={newType}
                        onChange={(v) => setNewType(v || "text")}
                        data={[
                            { value: "code", label: "Code" },
                            { value: "text", label: "Text" },
                            { value: "data", label: "Data" },
                        ]}
                    />
                    <Textarea
                        label={t("workspace.content")}
                        value={newContent}
                        onChange={(e) => setNewContent(e.currentTarget.value)}
                        minRows={4}
                        autosize
                        maxRows={12}
                    />
                    <Button onClick={handleAdd} disabled={!newName.trim()}>
                        {t("workspace.addArtifact")}
                    </Button>
                </Stack>
            </Modal>

            {/* Delete Confirm Modal */}
            <Modal opened={!!deleteId} onClose={() => setDeleteId(null)} title={t("workspace.deleteArtifact")} size="sm">
                <Text size="sm">{t("workspace.deleteConfirm")}</Text>
                <Group mt="md" justify="flex-end">
                    <Button variant="default" size="compact-sm" onClick={() => setDeleteId(null)}>
                        {t("common.cancel")}
                    </Button>
                    <Button color="red" size="compact-sm" onClick={handleDelete}>
                        {t("common.delete")}
                    </Button>
                </Group>
            </Modal>
        </Box>
    );
}

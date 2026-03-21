import { useEffect, useState } from "react";
import {
    Box,
    Text,
    Button,
    SimpleGrid,
    Card,
    Group,
    Badge,
    Modal,
    TextInput,
    Textarea,
    Stack,
    Menu,
    ActionIcon,
} from "@mantine/core";
import {
    IconNotebook,
    IconPlus,
    IconDots,
    IconPencil,
    IconTrash,
} from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import { useChatStore } from "../store/chatStore";

export default function NotebookPage() {
    const { t } = useTranslation();
    const notebooks = useChatStore((s) => s.notebooks);
    const loadNotebooks = useChatStore((s) => s.loadNotebooks);
    const createNotebook = useChatStore((s) => s.createNotebook);
    const updateNotebook = useChatStore((s) => s.updateNotebook);
    const deleteNotebook = useChatStore((s) => s.deleteNotebook);
    const setActiveNotebook = useChatStore((s) => s.setActiveNotebook);

    const [createOpen, setCreateOpen] = useState(false);
    const [createName, setCreateName] = useState("");
    const [createDesc, setCreateDesc] = useState("");

    const [renameId, setRenameId] = useState<string | null>(null);
    const [renameName, setRenameName] = useState("");
    const [renameDesc, setRenameDesc] = useState("");

    useEffect(() => {
        loadNotebooks();
    }, []);

    const handleCreate = async () => {
        if (!createName.trim()) return;
        const nb = await createNotebook(createName.trim(), createDesc.trim());
        if (nb) {
            setCreateOpen(false);
            setCreateName("");
            setCreateDesc("");
        }
    };

    const handleRename = async () => {
        if (!renameId || !renameName.trim()) return;
        await updateNotebook(renameId, renameName.trim(), renameDesc.trim());
        setRenameId(null);
    };

    const handleDelete = async (id: string, name: string) => {
        if (confirm(t("notebook.deleteConfirm", { name }))) {
            await deleteNotebook(id);
        }
    };

    const formatDate = (ts: number) => {
        return new Date(ts * 1000).toLocaleDateString();
    };

    return (
        <Box p="xl" style={{ height: "100%", overflow: "auto" }}>
            <Group justify="space-between" mb="xl">
                <Group gap="sm">
                    <IconNotebook size={28} stroke={1.5} color="var(--mantine-color-violet-5)" />
                    <Text size="xl" fw={700}>
                        {t("notebook.title")}
                    </Text>
                </Group>
                <Button
                    leftSection={<IconPlus size={16} />}
                    color="violet"
                    onClick={() => setCreateOpen(true)}
                >
                    {t("notebook.create")}
                </Button>
            </Group>

            {notebooks.length === 0 ? (
                <Box
                    style={{
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "center",
                        justifyContent: "center",
                        height: "60%",
                        gap: 16,
                    }}
                >
                    <IconNotebook size={64} stroke={1} color="var(--mantine-color-dimmed)" />
                    <Text c="dimmed" size="lg" ta="center">
                        {t("notebook.empty")}
                    </Text>
                    <Button
                        leftSection={<IconPlus size={16} />}
                        color="violet"
                        variant="light"
                        onClick={() => setCreateOpen(true)}
                    >
                        {t("notebook.create")}
                    </Button>
                </Box>
            ) : (
                <SimpleGrid cols={{ base: 1, xs: 2, sm: 3 }} spacing="md">
                    {notebooks.map((nb) => (
                        <Card
                            key={nb.id}
                            withBorder
                            p="md"
                            radius="md"
                            className="welcome-suggest-card"
                            style={{ cursor: "pointer" }}
                            onClick={() => setActiveNotebook(nb.id)}
                        >
                            <Group justify="space-between" mb="xs">
                                <Text fw={600} lineClamp={1} style={{ flex: 1 }}>
                                    {nb.name}
                                </Text>
                                <Menu position="bottom-end" withinPortal>
                                    <Menu.Target>
                                        <ActionIcon
                                            variant="subtle"
                                            size="sm"
                                            onClick={(e) => e.stopPropagation()}
                                        >
                                            <IconDots size={14} />
                                        </ActionIcon>
                                    </Menu.Target>
                                    <Menu.Dropdown>
                                        <Menu.Item
                                            leftSection={<IconPencil size={14} />}
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                setRenameId(nb.id);
                                                setRenameName(nb.name);
                                                setRenameDesc(nb.description);
                                            }}
                                        >
                                            {t("notebook.rename")}
                                        </Menu.Item>
                                        <Menu.Item
                                            leftSection={<IconTrash size={14} />}
                                            color="red"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                handleDelete(nb.id, nb.name);
                                            }}
                                        >
                                            {t("notebook.delete")}
                                        </Menu.Item>
                                    </Menu.Dropdown>
                                </Menu>
                            </Group>
                            {nb.description && (
                                <Text size="sm" c="dimmed" lineClamp={2} mb="xs">
                                    {nb.description}
                                </Text>
                            )}
                            <Group gap="xs" mt="auto">
                                <Badge size="sm" variant="light" color="violet">
                                    {t("notebook.documents", { count: nb.documentCount })}
                                </Badge>
                                <Badge size="sm" variant="light" color="gray">
                                    {t("notebook.chunks", { count: nb.totalChunks })}
                                </Badge>
                            </Group>
                            <Text size="xs" c="dimmed" mt="xs">
                                {formatDate(nb.updatedAt)}
                            </Text>
                        </Card>
                    ))}
                </SimpleGrid>
            )}

            {/* Create modal */}
            <Modal
                opened={createOpen}
                onClose={() => setCreateOpen(false)}
                title={t("notebook.createTitle")}
                centered
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("notebook.name")}
                        placeholder={t("notebook.namePlaceholder")}
                        value={createName}
                        onChange={(e) => setCreateName(e.currentTarget.value)}
                        autoFocus
                        onKeyDown={(e) => e.key === "Enter" && handleCreate()}
                    />
                    <Textarea
                        label={t("notebook.description")}
                        placeholder={t("notebook.descriptionPlaceholder")}
                        value={createDesc}
                        onChange={(e) => setCreateDesc(e.currentTarget.value)}
                        minRows={2}
                    />
                    <Button color="violet" onClick={handleCreate} disabled={!createName.trim()}>
                        {t("notebook.create")}
                    </Button>
                </Stack>
            </Modal>

            {/* Rename modal */}
            <Modal
                opened={renameId !== null}
                onClose={() => setRenameId(null)}
                title={t("notebook.rename")}
                centered
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("notebook.name")}
                        value={renameName}
                        onChange={(e) => setRenameName(e.currentTarget.value)}
                        autoFocus
                        onKeyDown={(e) => e.key === "Enter" && handleRename()}
                    />
                    <Textarea
                        label={t("notebook.description")}
                        value={renameDesc}
                        onChange={(e) => setRenameDesc(e.currentTarget.value)}
                        minRows={2}
                    />
                    <Button color="violet" onClick={handleRename} disabled={!renameName.trim()}>
                        {t("common.save")}
                    </Button>
                </Stack>
            </Modal>
        </Box>
    );
}

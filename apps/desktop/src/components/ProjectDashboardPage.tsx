import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Card,
    Collapse,
    Group,
    Menu,
    Progress,
    ScrollArea,
    Stack,
    Text,
    Textarea,
    TextInput,
    Title,
    Tooltip,
} from "@mantine/core";
import {
    IconArchive,
    IconArrowLeft,
    IconBrain,
    IconBriefcase,
    IconCheck,
    IconChevronDown,
    IconChevronRight,
    IconCode,
    IconDatabase,
    IconDots,
    IconFileText,
    IconMessage,
    IconPackage,
    IconSubtask,
    IconTrash,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { ConfirmModal } from "./ConfirmModal";
import { formatRelativeTime } from "../utils/formatDate";
import type { AgentMemory, AgentPlanWithProgress, WorkspaceArtifact } from "../types";

const STATUS_COLORS: Record<string, string> = {
    active: "green",
    completed: "blue",
    archived: "gray",
};

const CATEGORY_COLORS: Record<string, string> = {
    fact: "blue",
    decision: "violet",
    preference: "orange",
    context: "teal",
    learning: "green",
};

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

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ProjectDashboardPage() {
    const { t } = useTranslation();
    const setView = useChatStore((s) => s.setView);
    const activeProjectId = useChatStore((s) => s.activeProjectId);
    const projects = useChatStore((s) => s.projects);
    const chats = useChatStore((s) => s.chats);
    const updateProject = useChatStore((s) => s.updateProject);
    const deleteProject = useChatStore((s) => s.deleteProject);
    const archiveProject = useChatStore((s) => s.archiveProject);
    const setActiveChat = useChatStore((s) => s.setActiveChat);

    const project = useMemo(
        () => projects.find((p) => p.id === activeProjectId),
        [projects, activeProjectId],
    );

    // Editable fields
    const [editingName, setEditingName] = useState(false);
    const [nameValue, setNameValue] = useState("");
    const [editingGoal, setEditingGoal] = useState(false);
    const [goalValue, setGoalValue] = useState("");

    // Data
    const [artifacts, setArtifacts] = useState<WorkspaceArtifact[]>([]);
    const [memories, setMemories] = useState<AgentMemory[]>([]);
    const [memoryLimit, setMemoryLimit] = useState(10);
    const [plans, setPlans] = useState<AgentPlanWithProgress[]>([]);
    const [expandedArtifacts, setExpandedArtifacts] = useState<Set<string>>(new Set());

    // Delete
    const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);

    // Load data
    useEffect(() => {
        if (!activeProjectId) return;

        invoke<WorkspaceArtifact[]>("list_workspace_artifacts", {
            chatId: "",
            projectId: activeProjectId,
        })
            .then((list) => setArtifacts(list ?? []))
            .catch(console.error);

        invoke<AgentMemory[]>("list_agent_memories", {
            category: null,
            searchText: null,
            projectId: activeProjectId,
            limit: 100,
            offset: 0,
        })
            .then((list) => setMemories(list ?? []))
            .catch(console.error);

        invoke<AgentPlanWithProgress[]>("get_plans_for_project", {
            projectId: activeProjectId,
        })
            .then((list) => setPlans(list ?? []))
            .catch(console.error);
    }, [activeProjectId]);

    // Project chats
    const projectChats = useMemo(() => {
        if (!activeProjectId) return [];
        return chats
            .filter((c) => c.projectId === activeProjectId)
            .sort((a, b) => (b.updatedAt ?? b.createdAt) - (a.updatedAt ?? a.createdAt));
    }, [chats, activeProjectId]);

    const visibleMemories = useMemo(() => memories.slice(0, memoryLimit), [memories, memoryLimit]);

    const handleBack = useCallback(() => setView("chat"), [setView]);

    const startEditName = useCallback(() => {
        if (!project) return;
        setNameValue(project.name);
        setEditingName(true);
    }, [project]);

    const saveName = useCallback(async () => {
        if (!project || !nameValue.trim()) {
            setEditingName(false);
            return;
        }
        await updateProject(project.id, { name: nameValue.trim() });
        setEditingName(false);
    }, [project, nameValue, updateProject]);

    const startEditGoal = useCallback(() => {
        if (!project) return;
        setGoalValue(project.goal ?? "");
        setEditingGoal(true);
    }, [project]);

    const saveGoal = useCallback(async () => {
        if (!project) {
            setEditingGoal(false);
            return;
        }
        await updateProject(project.id, { goal: goalValue });
        setEditingGoal(false);
    }, [project, goalValue, updateProject]);

    const handleStatusChange = useCallback(
        async (status: string) => {
            if (!project) return;
            if (status === "archived") {
                await archiveProject(project.id);
            } else {
                await updateProject(project.id, { status });
            }
        },
        [project, updateProject, archiveProject],
    );

    const handleDelete = useCallback(async () => {
        if (!project) return;
        await deleteProject(project.id);
        setDeleteConfirmOpen(false);
        setView("chat");
    }, [project, deleteProject, setView]);

    const handleChatClick = useCallback(
        (chatId: string) => {
            void setActiveChat(chatId);
            setView("chat");
        },
        [setActiveChat, setView],
    );

    const toggleArtifact = useCallback((id: string) => {
        setExpandedArtifacts((prev) => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            return next;
        });
    }, []);

    if (!project) {
        return (
            <Box p="md" style={{ height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
                <Stack align="center" gap="sm">
                    <IconBriefcase size={48} stroke={1} style={{ opacity: 0.3 }} />
                    <Text c="dimmed">{t("project.dashboard.selectProject")}</Text>
                    <Button variant="light" onClick={handleBack}>
                        {t("project.dashboard.back")}
                    </Button>
                </Stack>
            </Box>
        );
    }

    const createdDate = new Date(project.createdAt * 1000).toLocaleDateString(
        undefined,
        { day: "numeric", month: "long", year: "numeric" },
    );

    return (
        <Box p="md" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
            {/* Header */}
            <Group justify="space-between" mb="md" wrap="nowrap">
                <Group gap="sm" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
                    <Tooltip label={t("project.dashboard.back")}>
                        <ActionIcon variant="subtle" size="lg" onClick={handleBack}>
                            <IconArrowLeft size={20} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>

                    {editingName ? (
                        <TextInput
                            value={nameValue}
                            onChange={(e) => setNameValue(e.currentTarget.value)}
                            onBlur={() => void saveName()}
                            onKeyDown={(e) => {
                                if (e.key === "Enter") void saveName();
                                if (e.key === "Escape") setEditingName(false);
                            }}
                            size="md"
                            style={{ flex: 1 }}
                            autoFocus
                        />
                    ) : (
                        <Title
                            order={3}
                            style={{ cursor: "pointer", flex: 1, minWidth: 0 }}
                            lineClamp={1}
                            onClick={startEditName}
                        >
                            {project.name}
                        </Title>
                    )}
                </Group>

                <Group gap="xs" wrap="nowrap">
                    <Menu position="bottom-end" withinPortal>
                        <Menu.Target>
                            <Badge
                                variant="light"
                                color={STATUS_COLORS[project.status] || "gray"}
                                size="lg"
                                style={{ cursor: "pointer" }}
                            >
                                {t(`project.status.${project.status}`)}
                            </Badge>
                        </Menu.Target>
                        <Menu.Dropdown>
                            <Menu.Item
                                leftSection={<IconCheck size={14} stroke={1.5} />}
                                onClick={() => void handleStatusChange("active")}
                            >
                                {t("project.status.active")}
                            </Menu.Item>
                            <Menu.Item
                                leftSection={<IconCheck size={14} stroke={1.5} />}
                                onClick={() => void handleStatusChange("completed")}
                            >
                                {t("project.status.completed")}
                            </Menu.Item>
                            <Menu.Item
                                leftSection={<IconArchive size={14} stroke={1.5} />}
                                onClick={() => void handleStatusChange("archived")}
                            >
                                {t("project.status.archived")}
                            </Menu.Item>
                        </Menu.Dropdown>
                    </Menu>

                    <Menu position="bottom-end" withinPortal>
                        <Menu.Target>
                            <ActionIcon variant="subtle" size="lg">
                                <IconDots size={20} stroke={1.5} />
                            </ActionIcon>
                        </Menu.Target>
                        <Menu.Dropdown>
                            <Menu.Item
                                leftSection={<IconArchive size={14} stroke={1.5} />}
                                onClick={() => void handleStatusChange("archived")}
                            >
                                {t("project.archive")}
                            </Menu.Item>
                            <Menu.Item
                                color="red"
                                leftSection={<IconTrash size={14} stroke={1.5} />}
                                onClick={() => setDeleteConfirmOpen(true)}
                            >
                                {t("project.delete")}
                            </Menu.Item>
                        </Menu.Dropdown>
                    </Menu>
                </Group>
            </Group>

            <ScrollArea style={{ flex: 1 }} offsetScrollbars>
                <Stack gap="lg" pb="md">
                    {/* Goal */}
                    <Box>
                        <Text size="sm" fw={500} mb={4}>
                            {t("project.goal")}
                        </Text>
                        {editingGoal ? (
                            <Textarea
                                value={goalValue}
                                onChange={(e) => setGoalValue(e.currentTarget.value)}
                                onBlur={() => void saveGoal()}
                                placeholder={t("project.dashboard.goalPlaceholder")}
                                minRows={2}
                                autosize
                                autoFocus
                            />
                        ) : (
                            <Text
                                size="sm"
                                c={project.goal ? undefined : "dimmed"}
                                style={{ cursor: "pointer" }}
                                onClick={startEditGoal}
                            >
                                {project.goal || t("project.dashboard.goalPlaceholder")}
                            </Text>
                        )}
                        <Text size="xs" c="dimmed" mt={4}>
                            {t("project.dashboard.created")}: {createdDate}
                        </Text>
                    </Box>

                    {/* Chats Section */}
                    <Box>
                        <Group justify="space-between" mb="xs">
                            <Group gap="xs">
                                <IconMessage size={18} stroke={1.5} />
                                <Text fw={500}>{t("project.dashboard.chats")}</Text>
                                <Badge variant="light" size="sm">
                                    {projectChats.length}
                                </Badge>
                            </Group>
                        </Group>

                        {projectChats.length === 0 ? (
                            <Text size="sm" c="dimmed" py="sm">
                                {t("project.dashboard.noChats")}
                            </Text>
                        ) : (
                            <Stack gap={4}>
                                {projectChats.map((chat) => (
                                    <Card
                                        key={chat.id}
                                        withBorder
                                        padding="xs"
                                        style={{ cursor: "pointer" }}
                                        onClick={() => handleChatClick(chat.id)}
                                    >
                                        <Group justify="space-between" wrap="nowrap">
                                            <Text size="sm" lineClamp={1} style={{ flex: 1, minWidth: 0 }}>
                                                {chat.title}
                                            </Text>
                                            <Group gap="xs" wrap="nowrap">
                                                <Text size="xs" c="dimmed">
                                                    {t("project.dashboard.messages", {
                                                        count: chat.messages.length,
                                                    })}
                                                </Text>
                                                <Text size="xs" c="dimmed">
                                                    {formatRelativeTime(chat.updatedAt ?? chat.createdAt)}
                                                </Text>
                                            </Group>
                                        </Group>
                                    </Card>
                                ))}
                            </Stack>
                        )}
                    </Box>

                    {/* Artifacts Section */}
                    <Box>
                        <Group gap="xs" mb="xs">
                            <IconPackage size={18} stroke={1.5} />
                            <Text fw={500}>{t("project.dashboard.artifacts")}</Text>
                            <Badge variant="light" size="sm">
                                {artifacts.length}
                            </Badge>
                        </Group>

                        {artifacts.length === 0 ? (
                            <Text size="sm" c="dimmed" py="sm">
                                {t("project.dashboard.noArtifacts")}
                            </Text>
                        ) : (
                            <Stack gap={4}>
                                {artifacts.map((artifact) => (
                                    <Card
                                        key={artifact.id}
                                        withBorder
                                        padding="xs"
                                        style={{ cursor: "pointer" }}
                                        onClick={() => toggleArtifact(artifact.id)}
                                    >
                                        <Group justify="space-between" wrap="nowrap">
                                            <Group gap="xs" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
                                                {expandedArtifacts.has(artifact.id) ? (
                                                    <IconChevronDown size={14} stroke={1.5} />
                                                ) : (
                                                    <IconChevronRight size={14} stroke={1.5} />
                                                )}
                                                {TYPE_ICONS[artifact.contentType] || (
                                                    <IconFileText size={14} stroke={1.5} />
                                                )}
                                                <Text size="sm" lineClamp={1} style={{ flex: 1, minWidth: 0 }}>
                                                    {artifact.name}
                                                </Text>
                                            </Group>
                                            <Group gap="xs" wrap="nowrap">
                                                <Badge
                                                    variant="light"
                                                    color={TYPE_COLORS[artifact.contentType] || "gray"}
                                                    size="xs"
                                                >
                                                    {artifact.contentType}
                                                </Badge>
                                                {artifact.content && (
                                                    <Text size="xs" c="dimmed">
                                                        {formatBytes(new TextEncoder().encode(artifact.content).length)}
                                                    </Text>
                                                )}
                                            </Group>
                                        </Group>
                                        <Collapse in={expandedArtifacts.has(artifact.id)}>
                                            <Box
                                                mt="xs"
                                                p="xs"
                                                style={{
                                                    background: "var(--mantine-color-dark-7)",
                                                    borderRadius: 4,
                                                    overflow: "auto",
                                                    maxHeight: 300,
                                                }}
                                            >
                                                <Text
                                                    size="xs"
                                                    style={{ whiteSpace: "pre-wrap", fontFamily: "monospace" }}
                                                >
                                                    {artifact.content}
                                                </Text>
                                            </Box>
                                        </Collapse>
                                    </Card>
                                ))}
                            </Stack>
                        )}
                    </Box>

                    {/* Memory Section */}
                    <Box>
                        <Group gap="xs" mb="xs">
                            <IconBrain size={18} stroke={1.5} />
                            <Text fw={500}>{t("project.dashboard.memory")}</Text>
                            <Badge variant="light" size="sm">
                                {memories.length}
                            </Badge>
                        </Group>

                        {memories.length === 0 ? (
                            <Text size="sm" c="dimmed" py="sm">
                                {t("project.dashboard.noMemory")}
                            </Text>
                        ) : (
                            <Stack gap={4}>
                                {visibleMemories.map((memory) => (
                                    <Card key={memory.id} withBorder padding="xs">
                                        <Group justify="space-between" wrap="nowrap" align="flex-start">
                                            <Text size="sm" lineClamp={2} style={{ flex: 1, minWidth: 0 }}>
                                                {memory.content}
                                            </Text>
                                            <Badge
                                                variant="light"
                                                color={CATEGORY_COLORS[memory.category] || "gray"}
                                                size="xs"
                                            >
                                                {memory.category}
                                            </Badge>
                                        </Group>
                                    </Card>
                                ))}
                                {memories.length > memoryLimit && (
                                    <Button
                                        variant="subtle"
                                        size="xs"
                                        onClick={() => setMemoryLimit((l) => l + 10)}
                                    >
                                        {t("project.dashboard.showMore")}
                                    </Button>
                                )}
                            </Stack>
                        )}
                    </Box>

                    {/* Plans Section */}
                    <Box>
                        <Group gap="xs" mb="xs">
                            <IconSubtask size={18} stroke={1.5} />
                            <Text fw={500}>{t("project.dashboard.plans")}</Text>
                            <Badge variant="light" size="sm">
                                {plans.length}
                            </Badge>
                        </Group>

                        {plans.length === 0 ? (
                            <Text size="sm" c="dimmed" py="sm">
                                {t("project.dashboard.noPlans")}
                            </Text>
                        ) : (
                            <Stack gap={4}>
                                {plans.map((plan) => {
                                    const pct =
                                        plan.totalTasks > 0
                                            ? Math.round((plan.completedTasks / plan.totalTasks) * 100)
                                            : 0;
                                    return (
                                        <Card
                                            key={plan.id}
                                            withBorder
                                            padding="xs"
                                            style={{ cursor: "pointer" }}
                                            onClick={() => handleChatClick(plan.chatId)}
                                        >
                                            <Group justify="space-between" wrap="nowrap" mb={4}>
                                                <Text size="sm" lineClamp={1} style={{ flex: 1, minWidth: 0 }}>
                                                    {plan.goal}
                                                </Text>
                                                <Group gap="xs" wrap="nowrap">
                                                    <Text size="xs" c="dimmed">
                                                        {t("project.dashboard.progress", {
                                                            completed: plan.completedTasks,
                                                            total: plan.totalTasks,
                                                        })}
                                                    </Text>
                                                    <Badge
                                                        variant="light"
                                                        color={
                                                            plan.status === "completed"
                                                                ? "green"
                                                                : plan.status === "failed"
                                                                  ? "red"
                                                                  : plan.status === "running"
                                                                    ? "blue"
                                                                    : "gray"
                                                        }
                                                        size="xs"
                                                    >
                                                        {plan.status}
                                                    </Badge>
                                                </Group>
                                            </Group>
                                            <Progress value={pct} size="sm" />
                                        </Card>
                                    );
                                })}
                            </Stack>
                        )}
                    </Box>
                </Stack>
            </ScrollArea>

            <ConfirmModal
                opened={deleteConfirmOpen}
                onClose={() => setDeleteConfirmOpen(false)}
                onConfirm={() => void handleDelete()}
                message={t("project.deleteConfirm")}
            />
        </Box>
    );
}

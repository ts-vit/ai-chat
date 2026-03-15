import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Collapse,
    Group,
    Loader,
    ScrollArea,
    Stack,
    Text,
    TextInput,
    Tooltip,
} from "@mantine/core";
import {
    IconCheck,
    IconChevronDown,
    IconChevronRight,
    IconPlayerPlay,
    IconPlayerSkipForward,
    IconRefresh,
    IconRoute,
    IconTrash,
    IconX,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { ConfirmModal } from "./ConfirmModal";
import type { AgentTask } from "../types";

const PLAN_STATUS_COLORS: Record<string, string> = {
    draft: "gray",
    approved: "blue",
    running: "blue",
    completed: "teal",
    failed: "red",
    paused: "yellow",
    cancelled: "gray",
};

function TaskStatusIcon({ status }: { status: string }) {
    if (status === "running") return <Loader size={14} />;
    if (status === "completed") return <IconCheck size={14} stroke={2} color="var(--mantine-color-teal-5)" />;
    if (status === "failed") return <IconX size={14} stroke={2} color="var(--mantine-color-red-5)" />;
    return (
        <Box
            style={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                backgroundColor: status === "skipped"
                    ? "var(--mantine-color-gray-5)"
                    : "var(--mantine-color-gray-4)",
                flexShrink: 0,
            }}
        />
    );
}

interface TaskCardProps {
    task: AgentTask;
    index: number;
    isDraft: boolean;
    isManual: boolean;
    canRun: boolean;
    allTasks: AgentTask[];
}

function TaskCard({ task, index, isDraft, isManual, canRun, allTasks }: TaskCardProps) {
    const { t } = useTranslation();
    const updatePlanTask = useChatStore((s) => s.updatePlanTask);
    const executeSingleTask = useChatStore((s) => s.executeSingleTask);
    const [expanded, setExpanded] = useState(false);
    const [editing, setEditing] = useState(false);
    const [editTitle, setEditTitle] = useState(task.title);
    const [editDesc, setEditDesc] = useState(task.description);

    const depNames = task.dependencies
        .map((depId) => {
            const dep = allTasks.find((t) => t.id === depId);
            return dep ? `${allTasks.indexOf(dep) + 1}` : "?";
        })
        .filter(Boolean);

    const handleSaveEdit = () => {
        updatePlanTask(task.id, { title: editTitle, description: editDesc });
        setEditing(false);
    };

    const handleRun = () => {
        executeSingleTask(task.id);
    };

    const handleSkip = () => {
        updatePlanTask(task.id, { status: "skipped" });
    };

    const handleRetry = () => {
        updatePlanTask(task.id, { status: "pending" });
        executeSingleTask(task.id);
    };

    return (
        <Box
            p="xs"
            style={{
                borderRadius: "var(--mantine-radius-sm)",
                backgroundColor: task.status === "running"
                    ? "var(--mantine-color-blue-light)"
                    : task.status === "failed"
                        ? "var(--mantine-color-red-light)"
                        : "var(--mantine-color-default-hover)",
                opacity: task.status === "skipped" ? 0.6 : 1,
            }}
        >
            <Group gap="xs" wrap="nowrap" align="flex-start">
                <Box mt={4}>
                    <TaskStatusIcon status={task.status} />
                </Box>
                <Box style={{ flex: 1, minWidth: 0 }}>
                    {editing ? (
                        <Stack gap={4}>
                            <TextInput
                                size="xs"
                                value={editTitle}
                                onChange={(e) => setEditTitle(e.currentTarget.value)}
                            />
                            <TextInput
                                size="xs"
                                value={editDesc}
                                onChange={(e) => setEditDesc(e.currentTarget.value)}
                            />
                            <Group gap={4}>
                                <Button size="compact-xs" onClick={handleSaveEdit}>OK</Button>
                                <Button size="compact-xs" variant="subtle" onClick={() => setEditing(false)}>
                                    {t("plans.skipTask")}
                                </Button>
                            </Group>
                        </Stack>
                    ) : (
                        <>
                            <Group gap={4} wrap="nowrap">
                                <Text
                                    size="sm"
                                    fw={500}
                                    lineClamp={1}
                                    style={{
                                        flex: 1,
                                        textDecoration: task.status === "skipped" ? "line-through" : undefined,
                                        cursor: isDraft ? "pointer" : undefined,
                                    }}
                                    onClick={isDraft ? () => {
                                        setEditTitle(task.title);
                                        setEditDesc(task.description);
                                        setEditing(true);
                                    } : undefined}
                                >
                                    {index + 1}. {task.title}
                                </Text>
                                {task.assignedModel != null && task.assignedModel !== "" && (
                                    <Badge
                                        size="xs"
                                        variant="light"
                                        color="gray"
                                        title={task.assignedModel}
                                        leftSection={<IconRoute size={14} stroke={1.5} />}
                                        style={{ flexShrink: 0 }}
                                    >
                                        {task.assignedModel.includes("/") ? (task.assignedModel.split("/").pop() ?? task.assignedModel).slice(0, 24) : task.assignedModel.includes(":") ? task.assignedModel.split(":").slice(1).join(":").slice(0, 24) : task.assignedModel.slice(0, 24)}
                                    </Badge>
                                )}
                            </Group>
                            {depNames.length > 0 && (
                                <Text size="xs" c="dimmed">
                                    {t("plans.dependsOn")}: {depNames.map((n) => `#${n}`).join(", ")}
                                </Text>
                            )}
                            {task.description && (
                                <>
                                    <Group
                                        gap={2}
                                        style={{ cursor: "pointer" }}
                                        onClick={() => setExpanded((o) => !o)}
                                    >
                                        {expanded ? <IconChevronDown size={12} /> : <IconChevronRight size={12} />}
                                        <Text size="xs" c="dimmed">
                                            {expanded ? "" : task.description.slice(0, 60) + (task.description.length > 60 ? "..." : "")}
                                        </Text>
                                    </Group>
                                    <Collapse in={expanded}>
                                        <Text size="xs" c="dimmed" mt={2}>
                                            {task.description}
                                        </Text>
                                    </Collapse>
                                </>
                            )}
                            {task.result && task.status === "completed" && (
                                <Text size="xs" c="dimmed" mt={2} lineClamp={2}>
                                    {t("plans.taskResult")}: {task.result.slice(0, 200)}
                                </Text>
                            )}
                        </>
                    )}
                </Box>
                {!editing && (
                    <Box>
                        {task.status === "failed" && (
                            <Tooltip label={t("plans.retryTask")}>
                                <ActionIcon size="xs" variant="subtle" color="red" onClick={handleRetry}>
                                    <IconRefresh size={14} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                        {isManual && canRun && task.status === "pending" && (
                            <Tooltip label={t("plans.runTask")}>
                                <ActionIcon size="xs" variant="subtle" color="brand" onClick={handleRun}>
                                    <IconPlayerPlay size={14} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                        {isDraft && task.status === "pending" && (
                            <Tooltip label={t("plans.skipTask")}>
                                <ActionIcon size="xs" variant="subtle" onClick={handleSkip}>
                                    <IconPlayerSkipForward size={14} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                    </Box>
                )}
            </Group>
        </Box>
    );
}

export function PlanPanel() {
    const { t } = useTranslation();
    const activePlan = useChatStore((s) => s.activePlan);
    const setShowPlanPanel = useChatStore((s) => s.setShowPlanPanel);
    const approvePlan = useChatStore((s) => s.approvePlan);
    const startPlanExecution = useChatStore((s) => s.startPlanExecution);
    const deletePlanAction = useChatStore((s) => s.deletePlan);
    const [deleteConfirm, setDeleteConfirm] = useState(false);

    if (!activePlan) return null;

    const { plan, tasks } = activePlan;
    const isDraft = plan.status === "draft";
    const isRunning = plan.status === "running";
    const isManual = plan.executionMode === "manual";
    const isCompleted = plan.status === "completed";
    const isFailed = plan.status === "failed";

    const completedIds = new Set(tasks.filter((t) => t.status === "completed").map((t) => t.id));
    const canRunTask = (task: AgentTask) =>
        task.dependencies.every((depId) => completedIds.has(depId));

    const completedCount = tasks.filter((t) => t.status === "completed").length;

    const handleRunAll = async () => {
        await approvePlan(plan.id, "auto");
        await startPlanExecution(plan.id);
    };

    const handleRunManual = async () => {
        await approvePlan(plan.id, "manual");
        await startPlanExecution(plan.id);
    };

    const handleDelete = () => {
        deletePlanAction(plan.id);
        setDeleteConfirm(false);
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
                <Box style={{ flex: 1, minWidth: 0 }}>
                    <Text size="sm" fw={600} lineClamp={2}>{plan.goal}</Text>
                    <Group gap={4} mt={2}>
                        <Badge size="xs" color={PLAN_STATUS_COLORS[plan.status] || "gray"}>
                            {t(`plans.status.${plan.status}` as const)}
                        </Badge>
                        <Text size="xs" c="dimmed">
                            {t("plans.progress", { completed: completedCount, total: tasks.length })}
                        </Text>
                    </Group>
                </Box>
                <ActionIcon size="sm" variant="subtle" onClick={() => setShowPlanPanel(false)}>
                    <IconX size={16} stroke={1.5} />
                </ActionIcon>
            </Group>

            {/* Status banner */}
            {isCompleted && (
                <Box p="xs" style={{ backgroundColor: "var(--mantine-color-teal-light)" }}>
                    <Text size="sm" fw={500} c="teal" ta="center">{t("plans.planCompleted")}</Text>
                </Box>
            )}
            {isFailed && (
                <Box p="xs" style={{ backgroundColor: "var(--mantine-color-red-light)" }}>
                    <Text size="sm" fw={500} c="red" ta="center">{t("plans.planFailed")}</Text>
                </Box>
            )}
            {plan.replanCount > 0 && (
                <Box p="xs" style={{ backgroundColor: "var(--mantine-color-yellow-light)" }}>
                    <Text size="xs" fw={500} c="yellow" ta="center">
                        {t("plans.replanCount", { count: plan.replanCount })}
                    </Text>
                </Box>
            )}
            {(() => {
                const runningCount = tasks.filter((t) => t.status === "running").length;
                return runningCount > 1 ? (
                    <Box p="xs" style={{ backgroundColor: "var(--mantine-color-blue-light)" }}>
                        <Text size="xs" fw={500} c="blue" ta="center">
                            {t("plans.parallelTasks", { count: runningCount })}
                        </Text>
                    </Box>
                ) : null;
            })()}

            {/* Tasks */}
            <ScrollArea style={{ flex: 1 }} p="xs">
                <Stack gap="xs">
                    {tasks
                        .sort((a, b) => a.sortOrder - b.sortOrder)
                        .map((task, i) => (
                            <TaskCard
                                key={task.id}
                                task={task}
                                index={i}
                                isDraft={isDraft}
                                isManual={isManual && isRunning}
                                canRun={canRunTask(task)}
                                allTasks={tasks}
                            />
                        ))}
                </Stack>
            </ScrollArea>

            {/* Actions */}
            <Stack gap={4} p="xs" style={{ borderTop: "1px solid var(--mantine-color-default-border)" }}>
                {isDraft && (
                    <>
                        <Button size="compact-sm" color="brand" onClick={handleRunAll}>
                            <IconPlayerPlay size={14} stroke={1.5} style={{ marginRight: 4 }} />
                            {t("plans.runAll")}
                        </Button>
                        <Button size="compact-sm" variant="light" color="brand" onClick={handleRunManual}>
                            {t("plans.runStepByStep")}
                        </Button>
                    </>
                )}
                {isManual && isRunning && (
                    <Button
                        size="compact-sm"
                        color="brand"
                        disabled={!tasks.some((t) => t.status === "pending" && canRunTask(t))}
                        onClick={() => {
                            const next = tasks
                                .sort((a, b) => a.sortOrder - b.sortOrder)
                                .find((t) => t.status === "pending" && canRunTask(t));
                            if (next) useChatStore.getState().executeSingleTask(next.id);
                        }}
                    >
                        <IconPlayerPlay size={14} stroke={1.5} style={{ marginRight: 4 }} />
                        {t("plans.runNext")}
                    </Button>
                )}
                <Button
                    size="compact-sm"
                    variant="subtle"
                    color="red"
                    onClick={() => setDeleteConfirm(true)}
                >
                    <IconTrash size={14} stroke={1.5} style={{ marginRight: 4 }} />
                    {t("plans.deletePlan")}
                </Button>
            </Stack>

            <ConfirmModal
                opened={deleteConfirm}
                onClose={() => setDeleteConfirm(false)}
                onConfirm={handleDelete}
                message={t("plans.confirmDelete")}
            />
        </Box>
    );
}

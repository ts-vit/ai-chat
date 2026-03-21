import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Card,
    Group,
    ScrollArea,
    Stack,
    Switch,
    Text,
    Title,
    Tooltip,
} from "@mantine/core";
import {
    IconCalendarEvent,
    IconPlayerPlay,
    IconEdit,
    IconTrash,
    IconPlus,
    IconBrandTelegram,
    IconBell,
    IconX,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { cronToHuman } from "../utils/cronHelper";
import { CreateScheduledTaskModal } from "./CreateScheduledTaskModal";
import type { ScheduledTask } from "../types";

function formatRelativeTime(ts: number | null, t: (key: string) => string): string {
    if (!ts) return t("scheduler.status.never");
    const now = Math.floor(Date.now() / 1000);
    const diff = ts - now;
    if (diff < 0) {
        const ago = Math.abs(diff);
        if (ago < 60) return `${ago}s ago`;
        if (ago < 3600) return `${Math.floor(ago / 60)}m ago`;
        if (ago < 86400) return `${Math.floor(ago / 3600)}h ago`;
        return new Date(ts * 1000).toLocaleDateString();
    }
    if (diff < 60) return `${diff}s`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
    return new Date(ts * 1000).toLocaleDateString();
}

function statusColor(status: string | null): string {
    if (status === "completed") return "green";
    if (status === "failed") return "red";
    if (status === "running") return "blue";
    return "gray";
}

export function SchedulerPage() {
    const { t, i18n } = useTranslation();
    const scheduledTasks = useChatStore((s) => s.scheduledTasks);
    const loadScheduledTasks = useChatStore((s) => s.loadScheduledTasks);
    const toggleScheduledTask = useChatStore((s) => s.toggleScheduledTask);
    const deleteScheduledTask = useChatStore((s) => s.deleteScheduledTask);
    const runScheduledTaskNow = useChatStore((s) => s.runScheduledTaskNow);

    const [modalOpen, setModalOpen] = useState(false);
    const [editingTask, setEditingTask] = useState<ScheduledTask | null>(null);

    useEffect(() => {
        loadScheduledTasks();
    }, [loadScheduledTasks]);

    const handleEdit = (task: ScheduledTask) => {
        setEditingTask(task);
        setModalOpen(true);
    };

    const handleCreate = () => {
        setEditingTask(null);
        setModalOpen(true);
    };

    const handleDelete = async (task: ScheduledTask) => {
        if (confirm(t("scheduler.deleteConfirm", { name: task.name }))) {
            await deleteScheduledTask(task.id);
        }
    };

    return (
        <Box p="md" h="100%" style={{ display: "flex", flexDirection: "column" }}>
            <Group justify="space-between" mb="md">
                <Group gap="sm">
                    <IconCalendarEvent size={24} stroke={1.5} />
                    <Title order={3}>{t("scheduler.title")}</Title>
                </Group>
                <Group gap="xs">
                    <Button
                        leftSection={<IconPlus size={16} stroke={1.5} />}
                        size="sm"
                        onClick={handleCreate}
                    >
                        {t("scheduler.newTask")}
                    </Button>
                    <Tooltip label={t("common.close")}>
                        <ActionIcon variant="subtle" size="lg" onClick={() => useChatStore.getState().setView("chat")}>
                            <IconX size={20} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Group>
            </Group>

            <ScrollArea style={{ flex: 1 }}>
                {scheduledTasks.length === 0 ? (
                    <Box ta="center" py="xl">
                        <Text c="dimmed" size="lg">{t("scheduler.noTasks")}</Text>
                        <Text c="dimmed" size="sm" mt="xs">{t("scheduler.noTasksHint")}</Text>
                    </Box>
                ) : (
                    <Stack gap="sm">
                        {scheduledTasks.map((task) => (
                            <Card key={task.id} padding="sm" radius="md" withBorder>
                                <Group justify="space-between" wrap="nowrap">
                                    <Box style={{ flex: 1, minWidth: 0 }}>
                                        <Group gap="xs" mb={4}>
                                            <Text fw={600} truncate>{task.name}</Text>
                                            <Badge
                                                size="xs"
                                                color={statusColor(task.lastRunStatus)}
                                                variant="light"
                                            >
                                                {task.lastRunStatus
                                                    ? t(`scheduler.status.${task.lastRunStatus}`)
                                                    : t("scheduler.status.never")}
                                            </Badge>
                                        </Group>

                                        <Text size="sm" c="dimmed" mb={2}>
                                            {cronToHuman(task.cronExpression, i18n.language)}
                                        </Text>

                                        <Text size="xs" c="dimmed" lineClamp={1}>
                                            {task.prompt}
                                        </Text>

                                        <Group gap="xs" mt={4}>
                                            {task.nextRunAt && task.enabled && (
                                                <Text size="xs" c="dimmed">
                                                    {t("scheduler.nextRun")}: {formatRelativeTime(task.nextRunAt, t)}
                                                </Text>
                                            )}
                                            {task.runCount > 0 && (
                                                <Text size="xs" c="dimmed">
                                                    {t("scheduler.runCount")}: {task.runCount} {t("scheduler.times")}
                                                </Text>
                                            )}
                                            {task.deliverTelegram && (
                                                <IconBrandTelegram size={14} stroke={1.5} style={{ opacity: 0.5 }} />
                                            )}
                                            {task.deliverDesktopNotification && (
                                                <IconBell size={14} stroke={1.5} style={{ opacity: 0.5 }} />
                                            )}
                                        </Group>

                                        {task.lastRunStatus === "failed" && task.lastRunError && (
                                            <Text size="xs" c="red" mt={2}>{task.lastRunError}</Text>
                                        )}
                                    </Box>

                                    <Group gap="xs" wrap="nowrap">
                                        <Switch
                                            checked={task.enabled}
                                            onChange={(e) => toggleScheduledTask(task.id, e.currentTarget.checked)}
                                            size="sm"
                                        />
                                        <Tooltip label={t("scheduler.runNow")}>
                                            <ActionIcon variant="subtle" size="sm" onClick={() => runScheduledTaskNow(task.id)}>
                                                <IconPlayerPlay size={16} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("scheduler.editTask")}>
                                            <ActionIcon variant="subtle" size="sm" onClick={() => handleEdit(task)}>
                                                <IconEdit size={16} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("scheduler.delete")}>
                                            <ActionIcon variant="subtle" size="sm" color="red" onClick={() => handleDelete(task)}>
                                                <IconTrash size={16} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                </Group>
                            </Card>
                        ))}
                    </Stack>
                )}
            </ScrollArea>

            <CreateScheduledTaskModal
                opened={modalOpen}
                onClose={() => { setModalOpen(false); setEditingTask(null); }}
                editTask={editingTask}
            />
        </Box>
    );
}

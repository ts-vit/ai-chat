import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Card,
    Group,
    Progress,
    ScrollArea,
    Stack,
    Text,
    Title,
    Tooltip,
} from "@mantine/core";
import {
    IconExternalLink,
    IconSubtask,
    IconTrash,
    IconX,
} from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { useChatStore } from "../store/chatStore";
import { ConfirmModal } from "./ConfirmModal";
import { formatRelativeTime } from "../utils/formatDate";
import type { AgentPlan, PlanWithTasks } from "../types";

const STATUS_COLORS: Record<string, string> = {
    draft: "gray",
    approved: "blue",
    running: "blue",
    completed: "teal",
    failed: "red",
    paused: "yellow",
    cancelled: "gray",
};

export function PlansPage() {
    const { t } = useTranslation();
    const allPlans = useChatStore((s) => s.allPlans);
    const loadAllPlans = useChatStore((s) => s.loadAllPlans);
    const deletePlan = useChatStore((s) => s.deletePlan);
    const setActiveChat = useChatStore((s) => s.setActiveChat);
    const setView = useChatStore((s) => s.setView);
    const setShowPlanPanel = useChatStore((s) => s.setShowPlanPanel);
    const chats = useChatStore((s) => s.chats);

    const [planDetails, setPlanDetails] = useState<Record<string, PlanWithTasks>>({});
    const [deleteId, setDeleteId] = useState<string | null>(null);

    useEffect(() => {
        loadAllPlans();
    }, [loadAllPlans]);

    useEffect(() => {
        const loadDetails = async () => {
            const details: Record<string, PlanWithTasks> = {};
            for (const plan of allPlans) {
                try {
                    const full = await invoke<PlanWithTasks>("get_plan", { planId: plan.id });
                    details[plan.id] = full;
                } catch {
                    // skip
                }
            }
            setPlanDetails(details);
        };
        if (allPlans.length > 0) loadDetails();
    }, [allPlans]);

    const handleOpenChat = (plan: AgentPlan) => {
        setActiveChat(plan.chatId);
        setShowPlanPanel(true);
        setView("chat");
    };

    const handleDelete = () => {
        if (deleteId) {
            deletePlan(deleteId);
            setDeleteId(null);
        }
    };

    return (
        <Box p="md" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
            <Group justify="space-between" mb="md">
                <Title order={3}>{t("plans.title")}</Title>
                <Tooltip label={t("common.close")}>
                    <ActionIcon variant="subtle" size="lg" onClick={() => setView("chat")}>
                        <IconX size={20} stroke={1.5} />
                    </ActionIcon>
                </Tooltip>
            </Group>

            {allPlans.length === 0 ? (
                <Box style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
                    <Stack align="center" gap="sm">
                        <IconSubtask size={48} stroke={1} color="var(--mantine-color-dimmed)" />
                        <Text c="dimmed" ta="center">{t("plans.empty")}</Text>
                    </Stack>
                </Box>
            ) : (
                <ScrollArea style={{ flex: 1 }}>
                    <Stack gap="sm">
                        {allPlans.map((plan) => {
                            const detail = planDetails[plan.id];
                            const tasks = detail?.tasks ?? [];
                            const completed = tasks.filter((t) => t.status === "completed").length;
                            const total = tasks.length;
                            const chatTitle = chats.find((c) => c.id === plan.chatId)?.title ?? "—";

                            return (
                                <Card key={plan.id} padding="sm" radius="md" withBorder>
                                    <Group justify="space-between" wrap="nowrap" mb={4}>
                                        <Text size="sm" fw={600} lineClamp={2} style={{ flex: 1 }}>
                                            {plan.goal}
                                        </Text>
                                        <Badge size="sm" color={STATUS_COLORS[plan.status] || "gray"}>
                                            {t(`plans.status.${plan.status}` as const)}
                                        </Badge>
                                    </Group>
                                    <Text size="xs" c="dimmed" mb={4}>
                                        {chatTitle} · {formatRelativeTime(plan.createdAt)}
                                    </Text>
                                    {total > 0 && (
                                        <Group gap="xs" mb={4}>
                                            <Progress
                                                value={total > 0 ? (completed / total) * 100 : 0}
                                                size="sm"
                                                color="teal"
                                                style={{ flex: 1 }}
                                            />
                                            <Text size="xs" c="dimmed">
                                                {t("plans.progress", { completed, total })}
                                            </Text>
                                        </Group>
                                    )}
                                    <Group gap={4} mt={4}>
                                        <Tooltip label={t("plans.openChat")}>
                                            <ActionIcon size="sm" variant="subtle" onClick={() => handleOpenChat(plan)}>
                                                <IconExternalLink size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("plans.deletePlan")}>
                                            <ActionIcon size="sm" variant="subtle" color="red" onClick={() => setDeleteId(plan.id)}>
                                                <IconTrash size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                </Card>
                            );
                        })}
                    </Stack>
                </ScrollArea>
            )}

            <ConfirmModal
                opened={!!deleteId}
                onClose={() => setDeleteId(null)}
                onConfirm={handleDelete}
                message={t("plans.confirmDelete")}
            />
        </Box>
    );
}

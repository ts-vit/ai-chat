import { ActionIcon, Box, Group, Text, Button, Progress, Badge, Tooltip } from "@mantine/core";
import { IconPlayerStop, IconPlayerPlay, IconLoader2, IconRoute, IconUsers, IconX, IconScissors, IconTimeline } from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import type { AgentStatus, SubAgentInfo } from "../types";

function formatRunCost(cost: number): string {
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    return `$${cost.toFixed(2)}`;
}

interface AgentStatusBarProps {
    iteration: number;
    maxIterations: number;
    status: AgentStatus;
    onCancel: () => void;
    onResume?: () => void;
    assignedModel?: string;
    runCost?: number;
    chatDefaultModel?: string;
    subAgents?: SubAgentInfo[];
    onCancelSubAgent?: (runId: string) => void;
    contextUsage?: { ratio: number; tokens: number; limit: number; trimmed: boolean } | null;
    traceStepCount?: number;
    showTracePanel?: boolean;
    onToggleTrace?: () => void;
}

export function AgentStatusBar({
    iteration,
    maxIterations,
    status,
    onCancel,
    onResume,
    assignedModel,
    runCost,
    chatDefaultModel,
    subAgents,
    onCancelSubAgent,
    contextUsage,
    traceStepCount,
    showTracePanel,
    onToggleTrace,
}: AgentStatusBarProps) {
    const { t } = useTranslation();
    const progress = maxIterations > 0 ? (iteration / maxIterations) * 100 : 0;
    const showModel =
        assignedModel && assignedModel !== chatDefaultModel;
    const showCost = runCost != null && runCost > 0;
    const activeSubAgents = subAgents?.filter((s) => s.status === "running") ?? [];

    if (status !== "running" && status !== "paused") return null;

    return (
        <Box
            style={{
                borderTop: "1px solid var(--mantine-color-default-border)",
                background: "var(--mantine-color-body)",
            }}
        >
            <Group
                gap="sm"
                px="md"
                py={6}
            >
                {status === "running" ? (
                    <>
                        <IconLoader2
                            size={16}
                            stroke={1.5}
                            style={{ animation: "spin 1s linear infinite" }}
                        />
                        <Text size="sm" c="dimmed">
                            {t("agent.working", { step: iteration, max: maxIterations })}
                        </Text>
                        {showModel && (
                            <Group gap={4} style={{ color: "var(--mantine-color-dimmed)" }}>
                                <IconRoute size={16} stroke={1.5} />
                                <Text size="xs" c="dimmed">
                                    {assignedModel.split("/").pop() ?? assignedModel}
                                </Text>
                            </Group>
                        )}
                        {showCost && (
                            <Text size="xs" c="dimmed">
                                {formatRunCost(runCost)}
                            </Text>
                        )}
                        <Progress
                            value={progress}
                            size="xs"
                            style={{ flex: 1 }}
                            color="orange"
                        />
                        {contextUsage && (
                            <Group gap={4}>
                                <Progress
                                    value={contextUsage.ratio * 100}
                                    size="xs"
                                    w={40}
                                    color={contextUsage.ratio > 0.85 ? "red" : contextUsage.ratio > 0.7 ? "yellow" : "blue"}
                                />
                                <Text size="xs" c="dimmed">
                                    {Math.round(contextUsage.ratio * 100)}%
                                </Text>
                                {contextUsage.trimmed && (
                                    <Tooltip label={t("agent.contextTrimmed")}>
                                        <IconScissors size={12} stroke={1.5} color="var(--mantine-color-yellow-6)" />
                                    </Tooltip>
                                )}
                            </Group>
                        )}
                        {traceStepCount != null && traceStepCount > 0 && onToggleTrace && (
                            <Tooltip label={t("agent.tracePanel")}>
                                <ActionIcon
                                    size="xs"
                                    variant={showTracePanel ? "filled" : "subtle"}
                                    color="orange"
                                    onClick={onToggleTrace}
                                >
                                    <IconTimeline size={14} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                        <Button
                            size="xs"
                            variant="subtle"
                            color="red"
                            onClick={onCancel}
                            leftSection={<IconPlayerStop size={14} stroke={1.5} />}
                        >
                            {t("agent.cancel")}
                        </Button>
                    </>
                ) : (
                    <>
                        <Text size="sm" c="dimmed">
                            {t("agent.paused", { step: iteration, max: maxIterations })}
                        </Text>
                        {showModel && (
                            <Group gap={4} style={{ color: "var(--mantine-color-dimmed)" }}>
                                <IconRoute size={16} stroke={1.5} />
                                <Text size="xs" c="dimmed">
                                    {assignedModel.split("/").pop() ?? assignedModel}
                                </Text>
                            </Group>
                        )}
                        {showCost && (
                            <Text size="xs" c="dimmed">
                                {formatRunCost(runCost!)}
                            </Text>
                        )}
                        {traceStepCount != null && traceStepCount > 0 && onToggleTrace && (
                            <Tooltip label={t("agent.tracePanel")}>
                                <ActionIcon
                                    size="xs"
                                    variant={showTracePanel ? "filled" : "subtle"}
                                    color="orange"
                                    onClick={onToggleTrace}
                                >
                                    <IconTimeline size={14} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                        {onResume && (
                            <Button
                                size="xs"
                                variant="subtle"
                                color="orange"
                                onClick={onResume}
                                leftSection={<IconPlayerPlay size={14} stroke={1.5} />}
                            >
                                {t("agent.continue")}
                            </Button>
                        )}
                        <Button
                            size="xs"
                            variant="subtle"
                            color="red"
                            onClick={onCancel}
                            leftSection={<IconPlayerStop size={14} stroke={1.5} />}
                        >
                            {t("agent.stop")}
                        </Button>
                    </>
                )}
            </Group>
            {activeSubAgents.length > 0 && (
                <Box px="md" pb={4}>
                    {activeSubAgents.map((sa) => (
                        <Group key={sa.agentRunId} gap={6} py={2}>
                            <IconUsers size={12} stroke={1.5} style={{ opacity: 0.5 }} />
                            <IconLoader2
                                size={12}
                                stroke={1.5}
                                style={{ animation: "spin 1s linear infinite", opacity: 0.6 }}
                            />
                            <Text size="xs" c="dimmed" lineClamp={1} style={{ flex: 1, minWidth: 0 }}>
                                {sa.goal}
                            </Text>
                            <Badge size="xs" variant="light" color="orange">
                                {sa.iterations}/{sa.maxIterations}
                            </Badge>
                            {sa.model && (
                                <Text size="xs" c="dimmed">
                                    {sa.model.split("/").pop() ?? sa.model}
                                </Text>
                            )}
                            {onCancelSubAgent && (
                                <Tooltip label={t("subAgent.cancel")}>
                                    <ActionIcon
                                        variant="subtle"
                                        size="xs"
                                        color="red"
                                        onClick={() => onCancelSubAgent(sa.agentRunId)}
                                    >
                                        <IconX size={12} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            )}
                        </Group>
                    ))}
                </Box>
            )}
        </Box>
    );
}

import {
    Accordion,
    Badge,
    Box,
    Group,
    ScrollArea,
    Stack,
    Text,
    Tooltip,
} from "@mantine/core";
import {
    IconBrain,
    IconChevronDown,
    IconTool,
    IconX,
} from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import type { AgentStepTrace } from "../types";

interface AgentTracePanelProps {
    steps: AgentStepTrace[];
}

function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
}

function formatTokens(n: number): string {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
    return String(n);
}

function formatCost(cost: number): string {
    if (cost === 0) return "$0";
    if (cost < 0.001) return `$${cost.toFixed(6)}`;
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    return `$${cost.toFixed(3)}`;
}

export default function AgentTracePanel({ steps }: AgentTracePanelProps) {
    const { t } = useTranslation();

    if (steps.length === 0) {
        return (
            <Box p="xs" style={{ opacity: 0.5 }}>
                <Text size="xs">{t("agent.traceNoTrace")}</Text>
            </Box>
        );
    }

    const totalDuration = steps.reduce((sum, s) => sum + s.durationMs, 0);
    const totalInputTokens = steps.reduce((sum, s) => sum + s.llmCall.inputTokens, 0);
    const totalOutputTokens = steps.reduce((sum, s) => sum + s.llmCall.outputTokens, 0);
    const totalCost = steps.reduce((sum, s) => sum + s.llmCall.cost, 0);
    const totalToolCalls = steps.reduce((sum, s) => sum + s.toolCalls.length, 0);

    return (
        <Box
            style={{
                borderTop: "1px solid var(--mantine-color-dark-4)",
                background: "var(--mantine-color-dark-7)",
            }}
        >
            {/* Summary */}
            <Group p="xs" gap="xs" wrap="nowrap">
                <Text size="xs" fw={600} c="dimmed">
                    {t("agent.tracePanel")}
                </Text>
                <Badge size="xs" variant="light" color="gray">
                    {steps.length} {t("agent.traceSteps")}
                </Badge>
                <Badge size="xs" variant="light" color="blue">
                    {formatTokens(totalInputTokens)} in / {formatTokens(totalOutputTokens)} out
                </Badge>
                {totalToolCalls > 0 && (
                    <Badge size="xs" variant="light" color="orange">
                        {totalToolCalls} tools
                    </Badge>
                )}
                <Badge size="xs" variant="light" color="gray">
                    {formatDuration(totalDuration)}
                </Badge>
                {totalCost > 0 && (
                    <Badge size="xs" variant="light" color="green">
                        {formatCost(totalCost)}
                    </Badge>
                )}
            </Group>

            {/* Steps */}
            <ScrollArea.Autosize mah={300} offsetScrollbars>
                <Accordion
                    variant="contained"
                    chevron={<IconChevronDown size={12} />}
                    styles={{
                        item: { borderBottom: "none", background: "transparent" },
                        control: { padding: "4px 8px", minHeight: 28 },
                        panel: { padding: "0 8px 4px 8px" },
                        chevron: { width: 16, minWidth: 16 },
                    }}
                >
                    {steps.map((step) => (
                        <Accordion.Item key={step.step} value={String(step.step)}>
                            <Accordion.Control>
                                <Group gap={6} wrap="nowrap">
                                    <Text size="xs" fw={500}>
                                        {t("agent.traceStep", { step: step.step })}
                                    </Text>
                                    <Badge size="xs" variant="dot" color="blue">
                                        {formatDuration(step.durationMs)}
                                    </Badge>
                                    <Badge size="xs" variant="light" color="gray">
                                        {formatTokens(step.llmCall.inputTokens)}/{formatTokens(step.llmCall.outputTokens)}
                                    </Badge>
                                    {step.toolCalls.length > 0 && (
                                        <Badge size="xs" variant="light" color="orange">
                                            {step.toolCalls.length} tool{step.toolCalls.length > 1 ? "s" : ""}
                                        </Badge>
                                    )}
                                </Group>
                            </Accordion.Control>
                            <Accordion.Panel>
                                <Stack gap={4}>
                                    {/* LLM Call */}
                                    <Group gap={4} wrap="nowrap">
                                        <IconBrain size={12} stroke={1.5} style={{ color: "var(--mantine-color-blue-5)", flexShrink: 0 }} />
                                        <Text size="xs" c="dimmed">
                                            {t("agent.traceLlmCall")}: {step.llmCall.inputTokens} in / {step.llmCall.outputTokens} out
                                            {step.llmCall.cost > 0 && ` — ${formatCost(step.llmCall.cost)}`}
                                            {` — ${formatDuration(step.llmCall.durationMs)}`}
                                        </Text>
                                    </Group>
                                    {step.llmCall.responsePreview && (
                                        <Text size="xs" c="dimmed" lineClamp={2} pl={16} style={{ opacity: 0.7 }}>
                                            {step.llmCall.responsePreview}
                                        </Text>
                                    )}

                                    {/* Tool Calls */}
                                    {step.toolCalls.map((tc, i) => (
                                        <Box key={i}>
                                            <Group gap={4} wrap="nowrap">
                                                {tc.isError ? (
                                                    <IconX size={12} stroke={1.5} style={{ color: "var(--mantine-color-red-5)", flexShrink: 0 }} />
                                                ) : (
                                                    <IconTool size={12} stroke={1.5} style={{ color: tc.isBuiltin ? "var(--mantine-color-orange-5)" : "var(--mantine-color-gray-5)", flexShrink: 0 }} />
                                                )}
                                                <Text size="xs" fw={500} c={tc.isError ? "red" : undefined}>
                                                    {tc.toolName}
                                                </Text>
                                                <Badge size="xs" variant="dot" color={tc.isError ? "red" : "gray"}>
                                                    {formatDuration(tc.durationMs)}
                                                </Badge>
                                                {tc.isBuiltin && (
                                                    <Badge size="xs" variant="light" color="orange">
                                                        {t("agent.traceBuiltin")}
                                                    </Badge>
                                                )}
                                            </Group>
                                            {tc.argumentsPreview && (
                                                <Tooltip label={tc.argumentsPreview} multiline maw={400} withArrow>
                                                    <Text size="xs" c="dimmed" lineClamp={1} pl={16} style={{ opacity: 0.6, cursor: "help" }}>
                                                        args: {tc.argumentsPreview}
                                                    </Text>
                                                </Tooltip>
                                            )}
                                            {tc.resultPreview && (
                                                <Tooltip label={tc.resultPreview} multiline maw={400} withArrow>
                                                    <Text size="xs" c="dimmed" lineClamp={1} pl={16} style={{ opacity: 0.6, cursor: "help" }}>
                                                        result: {tc.resultPreview}
                                                    </Text>
                                                </Tooltip>
                                            )}
                                        </Box>
                                    ))}
                                </Stack>
                            </Accordion.Panel>
                        </Accordion.Item>
                    ))}
                </Accordion>
            </ScrollArea.Autosize>
        </Box>
    );
}

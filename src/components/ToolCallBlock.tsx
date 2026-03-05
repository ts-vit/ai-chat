import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Box, Collapse, Group, Paper, Text, UnstyledButton } from "@mantine/core";
import { IconCheck, IconChevronDown, IconChevronRight, IconLoader2, IconX } from "@tabler/icons-react";
import type { ToolCallInfo } from "../types";

interface Props {
    toolCalls: ToolCallInfo[];
}

function ToolCallItem({ tc }: { tc: ToolCallInfo }) {
    const { t } = useTranslation();
    const [opened, setOpened] = useState(false);

    const argsPreview =
        tc.arguments.length > 100 ? tc.arguments.slice(0, 100) + "..." : tc.arguments;

    return (
        <Paper p="xs" withBorder radius="md" className="tool-call-block">
            <Group gap="xs" wrap="nowrap">
                {tc.status === "calling" && (
                    <IconLoader2 size={14} stroke={1.5} className="tool-call-spinner" />
                )}
                {tc.status === "done" && (
                    <IconCheck size={14} stroke={1.5} color="var(--mantine-color-green-6)" />
                )}
                {tc.status === "error" && (
                    <IconX size={14} stroke={1.5} color="var(--mantine-color-red-6)" />
                )}
                <Text size="xs" fw={500} lineClamp={1} style={{ flex: 1 }}>
                    {tc.status === "calling"
                        ? t("chat.mcpCallingTool", { name: tc.toolName })
                        : tc.toolName}
                </Text>
                {tc.status !== "calling" && (
                    <Text size="xs" c={tc.status === "error" ? "red" : "green"}>
                        {tc.status === "error" ? t("chat.mcpToolError") : t("chat.mcpToolDone")}
                    </Text>
                )}
            </Group>

            {tc.status === "calling" && argsPreview && argsPreview !== "{}" && (
                <Text size="xs" c="dimmed" mt={4} lineClamp={2}>
                    {t("chat.mcpToolArguments")}: {argsPreview}
                </Text>
            )}

            {tc.status !== "calling" && tc.result != null && (
                <Box mt={4}>
                    <UnstyledButton onClick={() => setOpened((o) => !o)}>
                        <Group gap={4}>
                            {opened ? (
                                <IconChevronDown size={12} stroke={1.5} />
                            ) : (
                                <IconChevronRight size={12} stroke={1.5} />
                            )}
                            <Text size="xs" c="dimmed">
                                {opened ? t("chat.mcpToolHideResult") : t("chat.mcpToolShowResult")}
                            </Text>
                        </Group>
                    </UnstyledButton>
                    <Collapse in={opened}>
                        <Box
                            className="tool-call-result-content"
                            mt={4}
                            style={tc.isError ? { color: "var(--mantine-color-red-text)" } : undefined}
                        >
                            {tc.result}
                        </Box>
                    </Collapse>
                </Box>
            )}
        </Paper>
    );
}

export function ToolCallBlock({ toolCalls }: Props) {
    if (toolCalls.length === 0) return null;
    return (
        <Box mt="xs">
            {toolCalls.map((tc) => (
                <ToolCallItem key={tc.toolCallId} tc={tc} />
            ))}
        </Box>
    );
}

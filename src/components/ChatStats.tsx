import { Box, Group, Text, Tooltip } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { useChatStore } from "../store/chatStore";

interface ChatStatsProps {
    compact?: boolean;
    providerId?: string;
}

function formatTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return n.toLocaleString();
}

export function ChatStats({ compact = false, providerId = "openrouter" }: ChatStatsProps) {
    const { t } = useTranslation();
    const { chats, activeChatId, balance, models, settings } = useChatStore();
    const activeChat = chats.find((c) => c.id === activeChatId);
    const assistantMessages = activeChat?.messages.filter((m) => m.role === "assistant") ?? [];

    const contextTokens = (() => {
        const last = assistantMessages[assistantMessages.length - 1];
        return last?.promptTokens ?? 0;
    })();
    const currentModel = models.find((m) => m.id === settings.model);
    const contextMax = currentModel?.context_length ?? 128000;

    const totalTokens = assistantMessages.reduce(
        (sum, m) => sum + (m.promptTokens ?? 0) + (m.completionTokens ?? 0),
        0
    );
    const totalCost = assistantMessages.reduce((sum, m) => sum + (m.cost ?? 0), 0);

    const sep = (key: string) =>
        !compact ? (
            <Text key={key} component="span" size="xs" c="dark.3" style={{ whiteSpace: "nowrap" }}>
                {" | "}
            </Text>
        ) : null;

    const parts: React.ReactNode[] = [];
    const showBalanceAndCost = providerId === "openrouter";

    if (showBalanceAndCost && balance !== null) {
        parts.push(
            <Tooltip key="balance" label={t("chatStats.balance")}>
                <Text component="span" size="xs" style={{ whiteSpace: "nowrap" }}>
                    ${balance.remaining.toFixed(2)}
                </Text>
            </Tooltip>
        );
        parts.push(sep("sep1"));
    }

    parts.push(
        <Tooltip key="context" label={t("chatStats.context")}>
            <Text component="span" size="xs" style={{ whiteSpace: "nowrap" }}>
                {contextTokens.toLocaleString()}/{formatTokens(contextMax)}
            </Text>
        </Tooltip>
    );
    parts.push(sep("sep2"));
    parts.push(
        <Tooltip key="tokens" label={t("chatStats.tokensInChat")}>
            <Text component="span" size="xs" style={{ whiteSpace: "nowrap" }}>
                {totalTokens.toLocaleString()}
            </Text>
        </Tooltip>
    );
    if (showBalanceAndCost) {
        parts.push(sep("sep3"));
        parts.push(
            <Tooltip key="cost" label={t("chatStats.costOfChat")}>
                <Text component="span" size="xs" style={{ whiteSpace: "nowrap" }}>
                    ${totalCost.toFixed(4)}
                </Text>
            </Tooltip>
        );
    }

    return (
        <Box
            p="xs"
            style={{
                backgroundColor: "var(--mantine-color-default-hover)",
                borderTop: "1px solid var(--mantine-color-default-border)",
                flexWrap: "nowrap",
                overflow: "hidden",
            }}
        >
            <Group gap="xs" wrap="nowrap" justify="flex-start" style={{ flexWrap: "nowrap", overflow: "hidden" }}>
                {parts}
            </Group>
        </Box>
    );
}

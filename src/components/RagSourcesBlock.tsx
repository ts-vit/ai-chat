import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge, Box, Collapse, Group, Text, UnstyledButton } from "@mantine/core";
import { IconChevronDown, IconChevronRight, IconDatabase } from "@tabler/icons-react";
import type { RagSource } from "../types";

interface Props {
    sources: RagSource[];
}

export function RagSourcesBlock({ sources }: Props) {
    const { t } = useTranslation();
    const [opened, setOpened] = useState(false);

    if (!sources || sources.length === 0) return null;

    return (
        <Box mt="xs">
            <UnstyledButton onClick={() => setOpened((o) => !o)}>
                <Group gap={4}>
                    <IconDatabase size={16} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                    <Text size="sm" fw={500} c="dimmed">
                        {t("kb.sources")} ({sources.length})
                    </Text>
                    {opened ? (
                        <IconChevronDown size={14} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                    ) : (
                        <IconChevronRight size={14} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                    )}
                </Group>
            </UnstyledButton>
            <Collapse in={opened}>
                <Box mt="xs" ml="xs" style={{ borderLeft: "2px solid var(--mantine-color-default-border)", paddingLeft: 8 }}>
                    {sources.map((source, i) => (
                        <Box key={i} mb="xs">
                            <Group gap={6} wrap="nowrap">
                                <Text size="xs" fw={500}>{source.documentName}</Text>
                                <Badge size="xs" variant="light" color="gray">#{source.chunkIndex + 1}</Badge>
                                <Badge size="xs" variant="light" color="brand">{(source.score * 100).toFixed(0)}%</Badge>
                            </Group>
                            <Text size="xs" c="dimmed" lineClamp={3} mt={2}>
                                {source.content}
                            </Text>
                        </Box>
                    ))}
                </Box>
            </Collapse>
        </Box>
    );
}

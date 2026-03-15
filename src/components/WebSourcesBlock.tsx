import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Anchor, Box, Collapse, Group, Text, UnstyledButton } from "@mantine/core";
import { IconChevronDown, IconChevronRight, IconWorld } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-shell";
import type { WebSource } from "../types";

interface Props {
    sources: WebSource[];
}

export function WebSourcesBlock({ sources }: Props) {
    const { t } = useTranslation();
    const [opened, setOpened] = useState(false);

    if (!sources || sources.length === 0) return null;

    return (
        <Box mt="xs">
            <UnstyledButton onClick={() => setOpened((o) => !o)}>
                <Group gap={4}>
                    <IconWorld size={16} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                    <Text size="sm" fw={500} c="dimmed">
                        {t("chat.sources")} ({sources.length})
                    </Text>
                    {opened ? (
                        <IconChevronDown size={14} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                    ) : (
                        <IconChevronRight size={14} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                    )}
                </Group>
            </UnstyledButton>
            <Collapse in={opened}>
                <Box mt="xs" ml="xs">
                    {sources.map((source, i) => (
                        <Box key={i} mb="xs">
                            <Anchor
                                size="sm"
                                onClick={(e) => {
                                    e.preventDefault();
                                    open(source.url).catch(console.error);
                                }}
                                style={{ cursor: "pointer" }}
                            >
                                {source.title || source.url}
                            </Anchor>
                            {source.snippet && (
                                <Text size="xs" c="dimmed" lineClamp={2}>
                                    {source.snippet}
                                </Text>
                            )}
                        </Box>
                    ))}
                </Box>
            </Collapse>
        </Box>
    );
}

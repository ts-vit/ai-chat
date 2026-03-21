import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge, Box, Collapse, Group, Progress, Text, UnstyledButton } from "@mantine/core";
import { IconChevronDown, IconChevronRight, IconDatabase } from "@tabler/icons-react";
import type { RagSource } from "../types";

interface Props {
    sources: RagSource[];
    citedIndices?: Set<number>;
}

export function RagSourcesBlock({ sources, citedIndices }: Props) {
    const { t } = useTranslation();
    const [opened, setOpened] = useState(false);
    const [highlightIndex, setHighlightIndex] = useState<number | null>(null);

    useEffect(() => {
        const handler = (e: Event) => {
            setOpened(true);
            const detail = (e as CustomEvent).detail;
            if (detail?.index) {
                setHighlightIndex(detail.index);
                setTimeout(() => setHighlightIndex(null), 2000);
            }
        };
        document.addEventListener("expand-rag-sources", handler);
        return () => document.removeEventListener("expand-rag-sources", handler);
    }, []);

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
                    {sources.map((source) => {
                        const isCited = !citedIndices || citedIndices.size === 0 || citedIndices.has(source.index);
                        const isHighlighted = highlightIndex === source.index;
                        return (
                            <Box
                                key={source.index}
                                id={`rag-source-${source.index}`}
                                mb="xs"
                                style={{
                                    opacity: isCited ? 1 : 0.5,
                                    transition: "background-color 0.3s ease, opacity 0.3s ease",
                                    backgroundColor: isHighlighted ? "var(--mantine-color-brand-light)" : undefined,
                                    borderRadius: 4,
                                    padding: isHighlighted ? "2px 4px" : undefined,
                                }}
                            >
                                <Group gap={6} wrap="nowrap">
                                    <Badge size="xs" variant="filled" color="brand">{source.index}</Badge>
                                    <Text size="xs" fw={500}>{source.documentName}</Text>
                                    <Badge size="xs" variant="light" color="gray">#{source.chunkIndex + 1}</Badge>
                                    <Progress value={source.score * 100} size="xs" w={50}
                                        color={source.score > 0.8 ? "green" : source.score > 0.5 ? "yellow" : "red"} />
                                    <Text size="xs" c="dimmed">{(source.score * 100).toFixed(0)}%</Text>
                                    {citedIndices && citedIndices.size > 0 && !isCited && (
                                        <Text size="xs" c="dimmed" fs="italic">{t("kb.sourceNotCited")}</Text>
                                    )}
                                </Group>
                                <Text size="xs" c="dimmed" lineClamp={3} mt={2}>
                                    {source.content}
                                </Text>
                            </Box>
                        );
                    })}
                </Box>
            </Collapse>
        </Box>
    );
}

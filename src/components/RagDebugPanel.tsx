import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionIcon, Badge, Box, Collapse, Group, Paper, Progress, Stack, Text } from "@mantine/core";
import { IconAnalyze } from "@tabler/icons-react";
import type { RagTrace } from "../types";
import { extractCitationIndices } from "../utils/citationParser";

interface Props {
    trace: RagTrace;
    sourcesCount: number;
    messageContent: string;
}

export function RagDebugPanel({ trace, sourcesCount, messageContent }: Props) {
    const { t } = useTranslation();
    const [opened, setOpened] = useState(false);

    const citedIndices = extractCitationIndices(messageContent);
    const opt = trace.optimization;
    const tokenSavings = opt ? Math.round((1 - opt.finalTokens / Math.max(opt.originalTokens, 1)) * 100) : 0;

    return (
        <Box mt={4}>
            <ActionIcon
                size="xs"
                variant="subtle"
                onClick={() => setOpened((o) => !o)}
                title={t("kb.ragDebug")}
            >
                <IconAnalyze size={14} stroke={1.5} />
            </ActionIcon>
            <Collapse in={opened}>
                <Paper withBorder p="xs" mt={4} style={{ fontSize: "var(--mantine-font-size-xs)" }}>
                    <Stack gap="xs">
                        {/* Query Processing */}
                        <Box>
                            <Group gap={4} mb={4}>
                                <Text size="xs" fw={600}>{t("kb.ragDebugQuery")}</Text>
                                <Badge size="xs" variant="light">{trace.variantsUsed.length} {t("kb.ragDebugVariants").toLowerCase()}</Badge>
                            </Group>
                            <Text size="xs" c="dimmed" mb={2}>{t("kb.ragDebugOriginal")}: {trace.originalQuery}</Text>
                            {trace.variantsUsed.length > 1 && (
                                <Group gap={4} mb={2}>
                                    {trace.variantsUsed.map((v, i) => (
                                        <Badge key={i} size="xs" variant="outline">{v.length > 60 ? v.slice(0, 60) + "..." : v}</Badge>
                                    ))}
                                </Group>
                            )}
                            {trace.subQuestions.length > 0 && (
                                <Box>
                                    <Text size="xs" c="dimmed">{t("kb.ragDebugSubQuestions")}:</Text>
                                    {trace.subQuestions.map((q, i) => (
                                        <Text key={i} size="xs" c="dimmed" ml="xs">• {q}</Text>
                                    ))}
                                </Box>
                            )}
                        </Box>

                        {/* Retrieval */}
                        <Box>
                            <Text size="xs" fw={600} mb={4}>{t("kb.ragDebugRetrieval")}</Text>
                            <Group gap="xs">
                                <Text size="xs">{t("kb.ragDebugCandidates")}: {trace.totalCandidates}</Text>
                                <Text size="xs" c="dimmed">→</Text>
                                <Text size="xs">{t("kb.ragDebugAfterDedup")}: {trace.afterDedup}</Text>
                                <Text size="xs" c="dimmed">→</Text>
                                <Text size="xs">{t("kb.ragDebugFinal")}: {trace.finalCount}</Text>
                            </Group>
                        </Box>

                        {/* Reranking */}
                        <Box>
                            <Text size="xs" fw={600} mb={4}>{t("kb.ragDebugReranking")}</Text>
                            <Group gap="xs">
                                {trace.rerankerUsed ? (
                                    <>
                                        <Badge size="xs" variant="filled" color="brand">{trace.rerankerUsed}</Badge>
                                        <Text size="xs">{trace.rerankCandidates} {t("kb.ragDebugCandidates").toLowerCase()}</Text>
                                    </>
                                ) : (
                                    <Badge size="xs" variant="light" color="gray">{t("kb.ragDebugDisabled")}</Badge>
                                )}
                            </Group>
                        </Box>

                        {/* Optimization */}
                        {opt && (
                            <Box>
                                <Group gap={4} mb={4}>
                                    <Text size="xs" fw={600}>{t("kb.ragDebugOptimization")}</Text>
                                    <Badge size="xs" variant="light" color={tokenSavings > 30 ? "green" : "yellow"}>-{tokenSavings}%</Badge>
                                </Group>
                                <Stack gap={2}>
                                    <Group gap="xs">
                                        <Text size="xs" w={160}>{t("kb.ragDebugOriginalTokens")}: {opt.originalTokens}</Text>
                                        <Progress value={100} size="xs" w={80} color="gray" />
                                    </Group>
                                    <Group gap="xs">
                                        <Text size="xs" w={160}>→ {t("kb.ragDebugSentenceExtraction")}: {opt.afterSentenceExtraction}</Text>
                                        <Progress value={opt.originalTokens > 0 ? (opt.afterSentenceExtraction / opt.originalTokens) * 100 : 0} size="xs" w={80} color="yellow" />
                                    </Group>
                                    <Group gap="xs">
                                        <Text size="xs" w={160}>→ {t("kb.ragDebugRedundancyRemoval")}: {opt.afterRedundancyRemoval}</Text>
                                        <Progress value={opt.originalTokens > 0 ? (opt.afterRedundancyRemoval / opt.originalTokens) * 100 : 0} size="xs" w={80} color="orange" />
                                    </Group>
                                    <Group gap="xs">
                                        <Text size="xs" w={160}>→ {t("kb.ragDebugFinalTokens")}: {opt.finalTokens}</Text>
                                        <Progress value={opt.originalTokens > 0 ? (opt.finalTokens / opt.originalTokens) * 100 : 0} size="xs" w={80} color="green" />
                                    </Group>
                                    <Text size="xs" c="dimmed">{t("kb.ragDebugChunks")}: {opt.chunksBefore} → {opt.chunksAfter}</Text>
                                </Stack>
                            </Box>
                        )}

                        {/* Citations */}
                        <Box>
                            <Text size="xs" fw={600} mb={4}>{t("kb.ragDebugCitations")}</Text>
                            <Group gap="xs">
                                <Text size="xs">{t("kb.ragDebugProvided")}: {sourcesCount}</Text>
                                <Text size="xs" c="dimmed">|</Text>
                                <Text size="xs">{t("kb.ragDebugCited")}: {citedIndices.length}</Text>
                            </Group>
                            {sourcesCount > 0 && citedIndices.length < sourcesCount && (
                                <Text size="xs" c="dimmed" mt={2}>
                                    {t("kb.ragDebugUncited")}: {Array.from({ length: sourcesCount }, (_, i) => i + 1)
                                        .filter((i) => !citedIndices.includes(i))
                                        .map((i) => `[${i}]`)
                                        .join(", ")}
                                </Text>
                            )}
                        </Box>
                    </Stack>
                </Paper>
            </Collapse>
        </Box>
    );
}

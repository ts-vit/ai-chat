import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Center,
    Group,
    Paper,
    Stack,
    Text,
    Title,
    Tooltip,
} from "@mantine/core";
import { IconArrowLeft, IconColumns, IconPlus, IconTrash } from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { formatRelativeDate } from "../utils/formatDate";
import { ConfirmModal } from "./ConfirmModal";
import { CreateComparisonModal } from "./CreateComparisonModal";

function shortModel(model: string): string {
    const name = model.split("/").pop() || model;
    return name.replace(/:free$/, "").replace(/:extended$/, "");
}

export function ComparisonsPage() {
    const { t } = useTranslation();
    const comparisons = useChatStore((s) => s.comparisons);
    const setActiveComparison = useChatStore((s) => s.setActiveComparison);
    const setView = useChatStore((s) => s.setView);
    const createComparison = useChatStore((s) => s.createComparison);
    const deleteComparison = useChatStore((s) => s.deleteComparison);

    const [createModalOpen, setCreateModalOpen] = useState(false);
    const [deletingId, setDeletingId] = useState<string | null>(null);

    const handleOpen = (id: string) => {
        setActiveComparison(id);
        setView("compare");
    };

    return (
        <Box p="md" style={{ height: "100vh", overflow: "auto" }}>
            <Stack gap="md" maw={800} mx="auto">
                <Group gap={4} wrap="nowrap" style={{ cursor: "pointer", flexShrink: 0 }} onClick={() => setView("chat")} mb="xs">
                    <IconArrowLeft size={16} stroke={1.5} color="var(--mantine-color-dimmed)" />
                    <Text size="sm" c="dimmed">{t("comparisons.backToChat")}</Text>
                </Group>
                <Group justify="space-between">
                    <Title order={3}>{t("comparisons.title")}</Title>
                    <Button
                        leftSection={<IconPlus size={16} stroke={1.5} />}
                        onClick={() => setCreateModalOpen(true)}
                    >
                        {t("comparisons.create")}
                    </Button>
                </Group>

                {comparisons.length === 0 ? (
                    <Center mih={300}>
                        <Stack align="center" gap="md">
                            <IconColumns size={48} stroke={1} color="var(--mantine-color-dimmed)" />
                            <Text c="dimmed">{t("comparisons.empty")}</Text>
                            <Button
                                variant="light"
                                leftSection={<IconPlus size={16} stroke={1.5} />}
                                onClick={() => setCreateModalOpen(true)}
                            >
                                {t("comparisons.create")}
                            </Button>
                        </Stack>
                    </Center>
                ) : (
                    <Stack gap="sm">
                        {comparisons.map((comp) => (
                            <Paper
                                key={comp.id}
                                p="md"
                                withBorder
                                style={{ cursor: "pointer" }}
                                onClick={() => handleOpen(comp.id)}
                            >
                                <Group justify="space-between" wrap="nowrap">
                                    <Stack gap={4} style={{ minWidth: 0 }}>
                                        <Text fw={500} lineClamp={1}>
                                            {comp.title || `${shortModel(comp.leftModel)} vs ${shortModel(comp.rightModel)}`}
                                        </Text>
                                        <Group gap="xs">
                                            <Badge size="xs" variant="light" color="brand">
                                                {shortModel(comp.leftModel)}
                                            </Badge>
                                            <Text size="xs" c="dimmed">vs</Text>
                                            <Badge size="xs" variant="light" color="grape">
                                                {shortModel(comp.rightModel)}
                                            </Badge>
                                            <Text size="xs" c="dimmed">
                                                {formatRelativeDate(comp.updatedAt)}
                                            </Text>
                                        </Group>
                                    </Stack>
                                    <Tooltip label={t("common.delete")}>
                                        <ActionIcon
                                            variant="subtle"
                                            color="red"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                setDeletingId(comp.id);
                                            }}
                                        >
                                            <IconTrash size={16} stroke={1.5} />
                                        </ActionIcon>
                                    </Tooltip>
                                </Group>
                            </Paper>
                        ))}
                    </Stack>
                )}
            </Stack>

            <CreateComparisonModal
                opened={createModalOpen}
                onClose={() => setCreateModalOpen(false)}
                onConfirm={(a, b, c, d, e, f) => {
                    createComparison(a, b, c, d, e, f);
                    setCreateModalOpen(false);
                }}
            />
            <ConfirmModal
                opened={deletingId !== null}
                onClose={() => setDeletingId(null)}
                onConfirm={() => {
                    if (deletingId) {
                        deleteComparison(deletingId);
                        setDeletingId(null);
                    }
                }}
                message={t("compare.deleteConfirm")}
            />
        </Box>
    );
}

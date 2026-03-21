import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Box, Text, SimpleGrid, Paper, Group, UnstyledButton, Stack, Badge } from "@mantine/core";
import {
    IconFileText,
    IconWorldSearch,
    IconSubtask,
    IconPencil,
    IconRobot,
    IconBrain,
    IconDatabase,
    IconWand,
    IconMessageCircle,
    IconCalendarEvent,
    IconFileImport,
} from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useChatStore } from "../store/chatStore";
import { formatRelativeTime } from "../utils/formatDate";
import { notify } from "../utils/notify";
import { TaskTemplateModal } from "./TaskTemplateModal";
import { TASK_TEMPLATES, type TaskTemplate, type TaskOptions } from "../constants/taskTemplates";

interface AssistantDashboardProps {
    onNewChat: () => void;
}

const DOC_FILTERS = [
    {
        name: "Documents",
        extensions: [
            "txt", "md", "html", "pdf", "docx", "json", "csv", "xml",
            "rst", "rs", "py", "ts", "tsx", "js", "jsx", "go", "java",
            "c", "cpp", "h", "rb", "php", "swift", "kt", "cs", "yaml",
            "yml", "toml",
        ],
    },
];

const SUPPORTED_EXTENSIONS = new Set([
    ".txt", ".md", ".html", ".htm", ".pdf", ".docx", ".json", ".csv", ".xml",
    ".rst", ".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".java",
    ".c", ".cpp", ".h", ".rb", ".php", ".swift", ".kt", ".cs", ".yaml",
    ".yml", ".toml",
]);

export function AssistantDashboard({ onNewChat }: AssistantDashboardProps) {
    const { t } = useTranslation();
    const [isDragOver, setIsDragOver] = useState(false);
    const [activeTemplate, setActiveTemplate] = useState<TaskTemplate | null>(null);

    const chats = useChatStore((s) => s.chats);
    const setView = useChatStore((s) => s.setView);
    const setActiveChat = useChatStore((s) => s.setActiveChat);
    const setInsertSnippetText = useChatStore((s) => s.setInsertSnippetText);

    const agentMemories = useChatStore((s) => s.agentMemories);
    const knowledgeBases = useChatStore((s) => s.knowledgeBases);
    const skills = useChatStore((s) => s.skills);
    const scheduledTasks = useChatStore((s) => s.scheduledTasks);

    const memoryCount = agentMemories.length;
    const kbCount = knowledgeBases.length;
    const skillCount = skills.length;

    const loadAgentMemories = useChatStore((s) => s.loadAgentMemories);
    const loadKnowledgeBases = useChatStore((s) => s.loadKnowledgeBases);
    const loadSkills = useChatStore((s) => s.loadSkills);
    const loadScheduledTasks = useChatStore((s) => s.loadScheduledTasks);
    const createKnowledgeBase = useChatStore((s) => s.createKnowledgeBase);
    const addKbDocuments = useChatStore((s) => s.addKbDocuments);
    const indexAllKbDocuments = useChatStore((s) => s.indexAllKbDocuments);
    const attachKbToChat = useChatStore((s) => s.attachKbToChat);

    useEffect(() => {
        loadAgentMemories();
        loadKnowledgeBases();
        loadSkills();
        loadScheduledTasks();
    }, []);

    // Drag & drop from OS file manager
    useEffect(() => {
        const unlistenDrop = listen<{ paths: string[] }>("tauri://drag-drop", (event) => {
            const paths = (event.payload as unknown as { paths: string[] }).paths;
            if (paths && paths.length > 0) {
                handleDocumentDrop(paths);
            }
            setIsDragOver(false);
        });

        const unlistenOver = listen("tauri://drag-over", () => {
            setIsDragOver(true);
        });

        const unlistenLeave = listen("tauri://drag-leave", () => {
            setIsDragOver(false);
        });

        return () => {
            unlistenDrop.then((f) => f());
            unlistenOver.then((f) => f());
            unlistenLeave.then((f) => f());
        };
    }, []);

    const recentChats = chats
        .filter((c) => (c.mode ?? "chat") === "assistant")
        .sort((a, b) => (b.updatedAt ?? b.createdAt) - (a.updatedAt ?? a.createdAt))
        .slice(0, 5);

    const recentMemories = agentMemories.slice(-2);

    const now = Math.floor(Date.now() / 1000);
    const nextTask = scheduledTasks
        .filter((t) => t.enabled && t.nextRunAt && t.nextRunAt > now)
        .sort((a, b) => (a.nextRunAt ?? 0) - (b.nextRunAt ?? 0))[0];

    const handleDocumentDrop = async (paths: string[]) => {
        const validPaths = paths.filter((p) => {
            const ext = p.substring(p.lastIndexOf(".")).toLowerCase();
            return SUPPORTED_EXTENSIONS.has(ext);
        });

        if (validPaths.length === 0) {
            notify.warning(t("assistantDashboard.unsupportedFiles"));
            return;
        }

        try {
            const name = `Documents ${new Date().toLocaleDateString()}`;
            const kb = await createKnowledgeBase(name, "Auto-created from dashboard drop");
            if (!kb) return;

            await addKbDocuments(kb.id, validPaths);
            await indexAllKbDocuments(kb.id);

            onNewChat();
            const chatId = useChatStore.getState().activeChatId;
            if (chatId) {
                await attachKbToChat(chatId, kb.id);
            }
            notify.success(t("assistantDashboard.documentsReady"));
        } catch (e) {
            notify.error(String(e));
        }
    };

    const handleAskDocument = useCallback(async () => {
        const selected = await open({ multiple: true, filters: DOC_FILTERS });
        if (!selected) return;
        const paths = Array.isArray(selected) ? selected : [selected];
        if (paths.length === 0) return;

        const name = `Documents ${new Date().toLocaleDateString()}`;
        const kb = await createKnowledgeBase(name, "Auto-created from dashboard");
        if (!kb) return;

        await addKbDocuments(kb.id, paths);
        await indexAllKbDocuments(kb.id);

        onNewChat();
        const chatId = useChatStore.getState().activeChatId;
        if (chatId) {
            await attachKbToChat(chatId, kb.id);
        }
    }, [createKnowledgeBase, addKbDocuments, indexAllKbDocuments, onNewChat, attachKbToChat]);

    const handleOpenTemplate = useCallback((templateId: string) => {
        const tmpl = TASK_TEMPLATES.find((t) => t.id === templateId) ?? null;
        setActiveTemplate(tmpl);
    }, []);

    const handleTemplateSubmit = useCallback((prompt: string, _options: TaskOptions) => {
        setActiveTemplate(null);
        onNewChat();
        setTimeout(() => setInsertSnippetText(prompt), 100);
    }, [onNewChat, setInsertSnippetText]);

    const actions = [
        { id: "ask_document", icon: IconFileText, color: "teal", titleKey: "assistantDashboard.askDocument", descKey: "assistantDashboard.askDocumentDesc", handler: handleAskDocument },
        { id: "research", icon: IconWorldSearch, color: "blue", titleKey: "assistantDashboard.research", descKey: "assistantDashboard.researchDesc", handler: () => handleOpenTemplate("research") },
        { id: "make_plan", icon: IconSubtask, color: "violet", titleKey: "assistantDashboard.makePlan", descKey: "assistantDashboard.makePlanDesc", handler: () => handleOpenTemplate("plan") },
        { id: "write_document", icon: IconPencil, color: "orange", titleKey: "assistantDashboard.writeDocument", descKey: "assistantDashboard.writeDocumentDesc", handler: () => handleOpenTemplate("write") },
        { id: "free_task", icon: IconRobot, color: "gray", titleKey: "assistantDashboard.freeTask", descKey: "assistantDashboard.freeTaskDesc", handler: onNewChat },
    ];

    return (
        <Box
            style={{
                flex: 1,
                display: "flex",
                flexDirection: "column",
                justifyContent: "center",
                alignItems: "center",
                height: "100%",
                padding: "24px",
                overflow: "auto",
                position: "relative",
            }}
        >
            {/* Drag overlay */}
            {isDragOver && (
                <Box
                    pos="absolute"
                    top={0} left={0} right={0} bottom={0}
                    style={{
                        background: "rgba(var(--mantine-color-teal-5-rgb, 32, 178, 170), 0.08)",
                        border: "2px dashed var(--mantine-color-teal-5)",
                        borderRadius: 8,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        zIndex: 100,
                    }}
                >
                    <Stack align="center" gap="xs">
                        <IconFileImport size={48} stroke={1.5} color="var(--mantine-color-teal-5)" />
                        <Text size="lg" fw={600} c="teal">
                            {t("assistantDashboard.dropDocuments")}
                        </Text>
                    </Stack>
                </Box>
            )}

            <Box style={{ maxWidth: 700, width: "100%" }}>
                <Box ta="center" mb="xl">
                    <IconRobot size={64} stroke={1} style={{ color: "var(--mantine-color-teal-5)", marginBottom: 8 }} />
                    <Text fw={700} size="xl" mb={4}>
                        UNI Assistant
                    </Text>
                    <Text size="sm" c="dimmed">
                        {t("modes.assistantWelcomeSubtitle")}
                    </Text>
                </Box>

                {/* Quick Actions */}
                <SimpleGrid cols={{ base: 1, xs: 2, sm: 3 }} spacing="sm" mb="lg">
                    {actions.map((a) => (
                        <Paper
                            key={a.id}
                            withBorder
                            p="md"
                            radius="md"
                            style={{ cursor: "pointer" }}
                            className="welcome-suggest-card"
                            onClick={a.handler}
                        >
                            <Group gap="sm" wrap="nowrap">
                                <a.icon size={28} stroke={1.5} style={{ color: `var(--mantine-color-${a.color}-5)`, flexShrink: 0 }} />
                                <Box>
                                    <Text size="sm" fw={600} lineClamp={1}>
                                        {t(a.titleKey)}
                                    </Text>
                                    <Text size="xs" c="dimmed" lineClamp={2}>
                                        {t(a.descKey)}
                                    </Text>
                                </Box>
                            </Group>
                        </Paper>
                    ))}
                </SimpleGrid>

                {/* Context Stats — Enriched */}
                <SimpleGrid cols={3} spacing="sm" mb="lg">
                    {/* Memory */}
                    <Paper
                        withBorder
                        p="sm"
                        radius="md"
                        style={{ cursor: "pointer" }}
                        className="welcome-suggest-card"
                        onClick={() => setView("memory")}
                    >
                        <Group gap="xs" mb={4} wrap="nowrap">
                            <IconBrain size={18} stroke={1.5} style={{ color: "var(--mantine-color-teal-5)", flexShrink: 0 }} />
                            <Text size="sm" fw={600} lineClamp={1} style={{ flex: 1 }}>
                                {t("assistantDashboard.contextMemories")}
                            </Text>
                            <Badge size="xs" variant="light" color="teal">{memoryCount}</Badge>
                        </Group>
                        {recentMemories.length > 0 ? (
                            <Stack gap={2}>
                                {recentMemories.map((m, i) => (
                                    <Text key={i} size="xs" c="dimmed" lineClamp={1}>
                                        {m.content}
                                    </Text>
                                ))}
                            </Stack>
                        ) : (
                            <Text size="xs" c="dimmed">{t("assistantDashboard.noMemory")}</Text>
                        )}
                    </Paper>

                    {/* Knowledge Bases */}
                    <Paper
                        withBorder
                        p="sm"
                        radius="md"
                        style={{ cursor: "pointer" }}
                        className="welcome-suggest-card"
                        onClick={() => setView("knowledgeBases")}
                    >
                        <Group gap="xs" mb={4} wrap="nowrap">
                            <IconDatabase size={18} stroke={1.5} style={{ color: "var(--mantine-color-teal-5)", flexShrink: 0 }} />
                            <Text size="sm" fw={600} lineClamp={1} style={{ flex: 1 }}>
                                {t("assistantDashboard.contextKBs")}
                            </Text>
                            <Badge size="xs" variant="light" color="teal">{kbCount}</Badge>
                        </Group>
                        {knowledgeBases.length > 0 ? (
                            <Stack gap={2}>
                                {knowledgeBases.slice(0, 2).map((kb) => (
                                    <Text key={kb.id} size="xs" c="dimmed" lineClamp={1}>
                                        {kb.name} — {kb.totalChunks ?? 0} chunks
                                    </Text>
                                ))}
                            </Stack>
                        ) : (
                            <Text size="xs" c="dimmed">{t("assistantDashboard.noKbPreview")}</Text>
                        )}
                    </Paper>

                    {/* Skills */}
                    <Paper
                        withBorder
                        p="sm"
                        radius="md"
                        style={{ cursor: "pointer" }}
                        className="welcome-suggest-card"
                        onClick={() => setView("skills")}
                    >
                        <Group gap="xs" mb={4} wrap="nowrap">
                            <IconWand size={18} stroke={1.5} style={{ color: "var(--mantine-color-teal-5)", flexShrink: 0 }} />
                            <Text size="sm" fw={600} lineClamp={1} style={{ flex: 1 }}>
                                {t("assistantDashboard.contextSkills")}
                            </Text>
                            <Badge size="xs" variant="light" color="teal">{skillCount}</Badge>
                        </Group>
                        {skills.length > 0 ? (
                            <Text size="xs" c="dimmed" lineClamp={2}>
                                {skills.slice(0, 4).map((s) => s.name).join(", ")}
                            </Text>
                        ) : (
                            <Text size="xs" c="dimmed">{t("assistantDashboard.noSkillsPreview")}</Text>
                        )}
                    </Paper>
                </SimpleGrid>

                {/* Next Scheduled Task */}
                {nextTask && (
                    <Paper
                        withBorder
                        p="sm"
                        radius="md"
                        mb="lg"
                        style={{ cursor: "pointer" }}
                        className="welcome-suggest-card"
                        onClick={() => setView("scheduler")}
                    >
                        <Group gap="xs" wrap="nowrap">
                            <IconCalendarEvent size={16} stroke={1.5} style={{ color: "var(--mantine-color-teal-5)", flexShrink: 0 }} />
                            <Text size="xs" c="dimmed">{t("assistantDashboard.nextScheduled")}</Text>
                            <Text size="xs" fw={500} lineClamp={1} style={{ flex: 1 }}>
                                {nextTask.name}
                            </Text>
                            <Text size="xs" c="dimmed" style={{ flexShrink: 0 }}>
                                {formatRelativeTime(nextTask.nextRunAt!)}
                            </Text>
                        </Group>
                    </Paper>
                )}

                {/* Recent Activity */}
                {recentChats.length > 0 && (
                    <Stack gap="xs">
                        <Text size="sm" fw={600} c="dimmed">
                            {t("assistantDashboard.recentActivity")}
                        </Text>
                        {recentChats.map((chat) => (
                            <UnstyledButton
                                key={chat.id}
                                onClick={() => setActiveChat(chat.id)}
                                style={{ borderRadius: "var(--mantine-radius-sm)", padding: "4px 8px" }}
                                className="welcome-suggest-card"
                            >
                                <Group gap="sm" wrap="nowrap">
                                    <IconMessageCircle size={16} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)", flexShrink: 0 }} />
                                    <Text size="sm" lineClamp={1} style={{ flex: 1 }}>
                                        {chat.title}
                                    </Text>
                                    <Text size="xs" c="dimmed" style={{ flexShrink: 0 }}>
                                        {formatRelativeTime(chat.updatedAt ?? chat.createdAt)}
                                    </Text>
                                </Group>
                            </UnstyledButton>
                        ))}
                    </Stack>
                )}
            </Box>

            <TaskTemplateModal
                opened={activeTemplate !== null}
                onClose={() => setActiveTemplate(null)}
                template={activeTemplate}
                onSubmit={handleTemplateSubmit}
            />
        </Box>
    );
}

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Box,
    Button,
    Checkbox,
    Group,
    Modal,
    Select,
    Stack,
    Text,
    Textarea,
    TextInput,
} from "@mantine/core";
import { useChatStore } from "../store/chatStore";
import { CRON_PRESETS, cronToHuman } from "../utils/cronHelper";
import type { ScheduledTask } from "../types";

interface Props {
    opened: boolean;
    onClose: () => void;
    editTask: ScheduledTask | null;
}

export function CreateScheduledTaskModal({ opened, onClose, editTask }: Props) {
    const { t, i18n } = useTranslation();
    const createScheduledTask = useChatStore((s) => s.createScheduledTask);
    const updateScheduledTask = useChatStore((s) => s.updateScheduledTask);
    const skills = useChatStore((s) => s.skills);
    const projects = useChatStore((s) => s.projects);
    const loadSkills = useChatStore((s) => s.loadSkills);

    const [name, setName] = useState("");
    const [prompt, setPrompt] = useState("");
    const [cronExpression, setCronExpression] = useState("0 9 * * *");
    const [model, setModel] = useState("");
    const [skillId, setSkillId] = useState<string | null>(null);
    const [projectId, setProjectId] = useState<string | null>(null);
    const [deliverTelegram, setDeliverTelegram] = useState(true);
    const [deliverDesktopNotification, setDeliverDesktopNotification] = useState(true);

    useEffect(() => {
        if (opened) {
            loadSkills();
        }
    }, [opened, loadSkills]);

    useEffect(() => {
        if (editTask) {
            setName(editTask.name);
            setPrompt(editTask.prompt);
            setCronExpression(editTask.cronExpression);
            setModel(editTask.model ?? "");
            setSkillId(editTask.skillId);
            setProjectId(editTask.projectId);
            setDeliverTelegram(editTask.deliverTelegram);
            setDeliverDesktopNotification(editTask.deliverDesktopNotification);
        } else {
            setName("");
            setPrompt("");
            setCronExpression("0 9 * * *");
            setModel("");
            setSkillId(null);
            setProjectId(null);
            setDeliverTelegram(true);
            setDeliverDesktopNotification(true);
        }
    }, [editTask, opened]);

    const handleSave = async () => {
        if (!name.trim() || !prompt.trim()) return;

        if (editTask) {
            await updateScheduledTask(editTask.id, {
                name: name.trim(),
                prompt: prompt.trim(),
                cronExpression,
                model: model.trim() || undefined,
                skillId: skillId || undefined,
                projectId: projectId || undefined,
                deliverTelegram,
                deliverDesktopNotification,
            });
        } else {
            await createScheduledTask({
                name: name.trim(),
                prompt: prompt.trim(),
                cronExpression,
                model: model.trim() || undefined,
                skillId: skillId || undefined,
                projectId: projectId || undefined,
                deliverTelegram,
                deliverDesktopNotification,
            });
        }
        onClose();
    };

    const isRu = i18n.language.startsWith("ru");
    const presetLabel = (p: typeof CRON_PRESETS[0]) => isRu ? p.labelRu : p.labelEn;

    return (
        <Modal
            opened={opened}
            onClose={onClose}
            title={editTask ? t("scheduler.editTask") : t("scheduler.newTask")}
            size="lg"
        >
            <Stack gap="sm">
                <TextInput
                    label={t("scheduler.name")}
                    placeholder={t("scheduler.namePlaceholder")}
                    value={name}
                    onChange={(e) => setName(e.currentTarget.value)}
                    required
                />

                <Textarea
                    label={t("scheduler.prompt")}
                    placeholder={t("scheduler.promptPlaceholder")}
                    value={prompt}
                    onChange={(e) => setPrompt(e.currentTarget.value)}
                    minRows={3}
                    maxRows={8}
                    autosize
                    required
                />

                <Box>
                    <Text size="sm" fw={500} mb={4}>{t("scheduler.presets")}</Text>
                    <Group gap="xs" wrap="wrap">
                        {CRON_PRESETS.map((preset) => (
                            <Button
                                key={preset.cron}
                                size="xs"
                                variant={cronExpression === preset.cron ? "filled" : "light"}
                                onClick={() => setCronExpression(preset.cron)}
                            >
                                {presetLabel(preset)}
                            </Button>
                        ))}
                    </Group>
                </Box>

                <TextInput
                    label={t("scheduler.cronExpression")}
                    placeholder="0 9 * * *"
                    value={cronExpression}
                    onChange={(e) => setCronExpression(e.currentTarget.value)}
                    description={cronToHuman(cronExpression, i18n.language)}
                />

                <TextInput
                    label={t("scheduler.model")}
                    placeholder="anthropic/claude-sonnet-4-20250514"
                    value={model}
                    onChange={(e) => setModel(e.currentTarget.value)}
                />

                {skills.length > 0 && (
                    <Select
                        label={t("scheduler.skill")}
                        placeholder="—"
                        data={skills.map((s) => ({ value: s.id, label: s.name }))}
                        value={skillId}
                        onChange={setSkillId}
                        clearable
                    />
                )}

                {projects.length > 0 && (
                    <Select
                        label={t("scheduler.project")}
                        placeholder="—"
                        data={projects.map((p) => ({ value: p.id, label: p.name }))}
                        value={projectId}
                        onChange={setProjectId}
                        clearable
                    />
                )}

                <Group gap="md">
                    <Checkbox
                        label={t("scheduler.deliverTelegram")}
                        checked={deliverTelegram}
                        onChange={(e) => setDeliverTelegram(e.currentTarget.checked)}
                    />
                    <Checkbox
                        label={t("scheduler.deliverDesktop")}
                        checked={deliverDesktopNotification}
                        onChange={(e) => setDeliverDesktopNotification(e.currentTarget.checked)}
                    />
                </Group>

                <Group justify="flex-end" mt="md">
                    <Button variant="default" onClick={onClose}>
                        {t("common.cancel")}
                    </Button>
                    <Button onClick={handleSave} disabled={!name.trim() || !prompt.trim()}>
                        {editTask ? t("common.save") : t("scheduler.newTask")}
                    </Button>
                </Group>
            </Stack>
        </Modal>
    );
}

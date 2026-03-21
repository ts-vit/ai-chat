import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Card,
    Group,
    Modal,
    Popover,
    ScrollArea,
    SimpleGrid,
    Stack,
    Switch,
    Text,
    Textarea,
    TextInput,
    Title,
    Tooltip,
} from "@mantine/core";
import {
    IconPencil,
    IconPlus,
    IconTrash,
    IconStar,
    IconX,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { ConfirmModal } from "./ConfirmModal";
import { SKILL_ICON_MAP, SKILL_ICON_OPTIONS } from "../constants/skillIcons";
import type { Skill } from "../types";
import type { FC } from "react";

function getSkillIcon(iconName: string): FC<{ size?: number; stroke?: number }> {
    return SKILL_ICON_MAP[iconName] || IconStar;
}

function SkillIconPicker({ value, onChange }: { value: string; onChange: (v: string) => void }) {
    const [opened, setOpened] = useState(false);
    const SelectedIcon = getSkillIcon(value);

    return (
        <Popover opened={opened} onChange={setOpened} position="bottom-start" width={280}>
            <Popover.Target>
                <Button
                    variant="default"
                    leftSection={<SelectedIcon size={18} stroke={1.5} />}
                    onClick={() => setOpened((o) => !o)}
                    size="sm"
                >
                    {value}
                </Button>
            </Popover.Target>
            <Popover.Dropdown>
                <SimpleGrid cols={6} spacing="xs">
                    {SKILL_ICON_OPTIONS.map((name) => {
                        const Icon = SKILL_ICON_MAP[name];
                        return (
                            <Tooltip key={name} label={name} openDelay={300}>
                                <ActionIcon
                                    variant={value === name ? "filled" : "subtle"}
                                    color={value === name ? "brand" : "gray"}
                                    size="lg"
                                    onClick={() => { onChange(name); setOpened(false); }}
                                >
                                    <Icon size={18} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        );
                    })}
                </SimpleGrid>
            </Popover.Dropdown>
        </Popover>
    );
}

interface SkillFormData {
    name: string;
    description: string;
    icon: string;
    content: string;
    triggerDescription: string;
    requiredTools: string;
}

const emptyForm: SkillFormData = {
    name: "",
    description: "",
    icon: "star",
    content: "",
    triggerDescription: "",
    requiredTools: "[]",
};

export function SkillsPage() {
    const { t } = useTranslation();
    const skills = useChatStore((s) => s.skills);
    const loadSkills = useChatStore((s) => s.loadSkills);
    const createSkill = useChatStore((s) => s.createSkill);
    const updateSkill = useChatStore((s) => s.updateSkill);
    const deleteSkill = useChatStore((s) => s.deleteSkill);

    const [modalOpen, setModalOpen] = useState(false);
    const [editingSkill, setEditingSkill] = useState<Skill | null>(null);
    const [form, setForm] = useState<SkillFormData>(emptyForm);
    const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

    useEffect(() => {
        loadSkills();
    }, [loadSkills]);

    const openCreate = () => {
        setEditingSkill(null);
        setForm(emptyForm);
        setModalOpen(true);
    };

    const openEdit = (skill: Skill) => {
        setEditingSkill(skill);
        setForm({
            name: skill.name,
            description: skill.description,
            icon: skill.icon,
            content: skill.content,
            triggerDescription: skill.triggerDescription,
            requiredTools: skill.requiredTools,
        });
        setModalOpen(true);
    };

    const handleSave = async () => {
        if (!form.name.trim()) return;
        if (editingSkill) {
            await updateSkill(editingSkill.id, {
                name: form.name,
                description: form.description,
                icon: form.icon,
                content: form.content,
                triggerDescription: form.triggerDescription,
                requiredTools: form.requiredTools,
            });
        } else {
            await createSkill(
                form.name,
                form.description,
                form.icon,
                form.content,
                form.triggerDescription,
                form.requiredTools,
            );
        }
        setModalOpen(false);
    };

    const handleDelete = async (id: string) => {
        await deleteSkill(id);
        setDeleteConfirm(null);
    };

    const handleToggle = async (skill: Skill) => {
        await updateSkill(skill.id, { enabled: !skill.enabled });
    };

    const sorted = [...skills].sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name));

    return (
        <Box p="md" h="100%" style={{ display: "flex", flexDirection: "column" }}>
            <Group justify="space-between" mb="md">
                <Title order={3}>{t("skills.title")}</Title>
                <Group gap="xs">
                    <Button
                        leftSection={<IconPlus size={16} stroke={1.5} />}
                        size="sm"
                        onClick={openCreate}
                    >
                        {t("skills.createSkill")}
                    </Button>
                    <Tooltip label={t("common.close")}>
                        <ActionIcon variant="subtle" size="lg" onClick={() => useChatStore.getState().setView("chat")}>
                            <IconX size={20} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Group>
            </Group>

            <ScrollArea style={{ flex: 1 }}>
                <Stack gap="sm">
                    {sorted.map((skill) => {
                        const Icon = getSkillIcon(skill.icon);
                        return (
                            <Card key={skill.id} padding="sm" radius="md" withBorder>
                                <Group justify="space-between" wrap="nowrap">
                                    <Group gap="sm" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
                                        <Icon size={24} stroke={1.5} />
                                        <Box style={{ flex: 1, minWidth: 0 }}>
                                            <Group gap="xs">
                                                <Text fw={500} truncate>
                                                    {skill.isBuiltin ? t(`skills.builtin.${skill.id}.name`, { defaultValue: skill.name }) : skill.name}
                                                </Text>
                                                {skill.isBuiltin && (
                                                    <Badge size="xs" variant="light" color="gray">
                                                        {t("skills.builtin")}
                                                    </Badge>
                                                )}
                                            </Group>
                                            <Text size="xs" c="dimmed" lineClamp={1}>
                                                {skill.isBuiltin ? t(`skills.builtin.${skill.id}.description`, { defaultValue: skill.description }) : skill.description}
                                            </Text>
                                        </Box>
                                    </Group>
                                    <Group gap="xs" wrap="nowrap">
                                        <Switch
                                            checked={skill.enabled}
                                            onChange={() => handleToggle(skill)}
                                            size="sm"
                                        />
                                        <Tooltip label={t("skills.editSkill")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="sm"
                                                onClick={() => openEdit(skill)}
                                            >
                                                <IconPencil size={16} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        {!skill.isBuiltin && (
                                            <Tooltip label={t("skills.deleteSkill")}>
                                                <ActionIcon
                                                    variant="subtle"
                                                    color="red"
                                                    size="sm"
                                                    onClick={() => setDeleteConfirm(skill.id)}
                                                >
                                                    <IconTrash size={16} stroke={1.5} />
                                                </ActionIcon>
                                            </Tooltip>
                                        )}
                                    </Group>
                                </Group>
                            </Card>
                        );
                    })}
                </Stack>
            </ScrollArea>

            {/* Create / Edit Modal */}
            <Modal
                opened={modalOpen}
                onClose={() => setModalOpen(false)}
                title={editingSkill ? t("skills.editSkill") : t("skills.createSkill")}
                size="lg"
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("skills.name")}
                        value={form.name}
                        onChange={(e) => setForm({ ...form, name: e.currentTarget.value })}
                        required
                    />
                    <TextInput
                        label={t("skills.description")}
                        value={form.description}
                        onChange={(e) => setForm({ ...form, description: e.currentTarget.value })}
                    />
                    <Box>
                        <Text size="sm" fw={500} mb={4}>{t("skills.icon")}</Text>
                        <SkillIconPicker value={form.icon} onChange={(v) => setForm({ ...form, icon: v })} />
                    </Box>
                    <Textarea
                        label={t("skills.content")}
                        description={t("skills.contentHelp")}
                        value={form.content}
                        onChange={(e) => setForm({ ...form, content: e.currentTarget.value })}
                        minRows={8}
                        maxRows={16}
                        autosize
                        styles={{ input: { fontFamily: "monospace", fontSize: "13px" } }}
                    />
                    <Textarea
                        label={t("skills.triggerDescription")}
                        description={t("skills.triggerDescriptionHelp")}
                        value={form.triggerDescription}
                        onChange={(e) => setForm({ ...form, triggerDescription: e.currentTarget.value })}
                        minRows={2}
                        maxRows={4}
                        autosize
                    />
                    <TextInput
                        label={t("skills.requiredTools")}
                        value={form.requiredTools}
                        onChange={(e) => setForm({ ...form, requiredTools: e.currentTarget.value })}
                        placeholder='["filesystem", "web_search"]'
                    />
                    <Group justify="flex-end" mt="md">
                        <Button variant="default" onClick={() => setModalOpen(false)}>
                            {t("skills.cancel")}
                        </Button>
                        <Button onClick={handleSave} disabled={!form.name.trim()}>
                            {t("skills.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            {/* Delete Confirm */}
            <ConfirmModal
                opened={!!deleteConfirm}
                onClose={() => setDeleteConfirm(null)}
                onConfirm={() => deleteConfirm && handleDelete(deleteConfirm)}
                title={t("skills.deleteSkill")}
                message={t("skills.deleteConfirm")}
            />
        </Box>
    );
}

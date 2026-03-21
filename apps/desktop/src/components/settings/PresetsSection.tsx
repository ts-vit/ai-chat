import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Button,
    Card,
    Checkbox,
    Group,
    Modal,
    Stack,
    Text,
    Textarea,
    TextInput,
    Tooltip,
} from "@mantine/core";
import { IconEdit, IconStar, IconStarFilled, IconTrash } from "@tabler/icons-react";
import { useChatStore } from "../../store/chatStore";
import type { Preset } from "../../types";
import { ConfirmModal } from "../ConfirmModal";

export function PresetsSection() {
    const { t } = useTranslation();
    const { presets, createPreset, updatePreset, deletePreset } = useChatStore();

    const [presetModalOpen, setPresetModalOpen] = useState(false);
    const [presetEditId, setPresetEditId] = useState<string | null>(null);
    const [presetName, setPresetName] = useState("");
    const [presetContent, setPresetContent] = useState("");
    const [presetIsDefault, setPresetIsDefault] = useState(false);
    const [deletingPresetId, setDeletingPresetId] = useState<string | null>(null);

    const openPresetModal = (preset?: Preset) => {
        if (preset) {
            setPresetEditId(preset.id);
            setPresetName(preset.name);
            setPresetContent(preset.content);
            setPresetIsDefault(preset.isDefault);
        } else {
            setPresetEditId(null);
            setPresetName("");
            setPresetContent("");
            setPresetIsDefault(false);
        }
        setPresetModalOpen(true);
    };

    const closePresetModal = () => {
        setPresetModalOpen(false);
        setPresetEditId(null);
        setPresetName("");
        setPresetContent("");
        setPresetIsDefault(false);
    };

    const savePresetFromModal = async () => {
        const name = presetName.trim();
        const content = presetContent.trim();
        if (!name) return;
        if (presetEditId) {
            await updatePreset(presetEditId, name, content, presetIsDefault);
        } else {
            await createPreset(name, content, presetIsDefault);
        }
        closePresetModal();
    };

    const setPresetAsDefault = (id: string) => {
        const p = presets.find((x) => x.id === id);
        if (p) updatePreset(p.id, p.name, p.content, true);
    };

    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Text size="sm" fw={500}>
                        {t("settings.presets.title")}
                    </Text>
                    <Button size="xs" variant="light" onClick={() => openPresetModal()}>
                        {t("settings.presets.addPreset")}
                    </Button>
                </Group>
                {presets.length === 0 ? (
                    <Text size="xs" c="dimmed">
                        {t("settings.presets.noPresets")}
                    </Text>
                ) : (
                    <Stack gap="xs">
                        {presets.map((p) => (
                            <Card key={p.id} withBorder padding="sm">
                                <Group justify="space-between" wrap="nowrap" align="flex-start">
                                    <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
                                        <Text size="sm" fw={p.isDefault ? 700 : 400}>
                                            {p.name}
                                        </Text>
                                        <Text size="xs" c="dimmed" lineClamp={1}>
                                            {p.content.slice(0, 100)}
                                            {p.content.length > 100 ? "…" : ""}
                                        </Text>
                                    </Stack>
                                    <Group gap="sm" wrap="nowrap">
                                        <Tooltip label={t("common.edit")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                onClick={() => openPresetModal(p)}
                                                aria-label={t("common.edit")}
                                            >
                                                <IconEdit size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("common.delete")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                color="red"
                                                onClick={() => setDeletingPresetId(p.id)}
                                                aria-label={t("common.delete")}
                                            >
                                                <IconTrash size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={p.isDefault ? t("settings.presets.defaultTooltip") : t("settings.presets.setDefaultTooltip")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                onClick={() => setPresetAsDefault(p.id)}
                                                aria-label={p.isDefault ? t("settings.presets.defaultTooltip") : t("settings.presets.setDefaultTooltip")}
                                            >
                                                {p.isDefault ? (
                                                    <IconStarFilled size={14} stroke={1.5} />
                                                ) : (
                                                    <IconStar size={14} stroke={1.5} />
                                                )}
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                </Group>
                            </Card>
                        ))}
                    </Stack>
                )}
            </Stack>

            <Modal
                title={presetEditId ? t("settings.presets.editPreset") : t("settings.presets.addPresetModal")}
                opened={presetModalOpen}
                onClose={closePresetModal}
                size="sm"
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("common.name")}
                        placeholder={t("settings.presets.namePlaceholder")}
                        value={presetName}
                        onChange={(e) => setPresetName(e.currentTarget.value)}
                    />
                    <Textarea
                        label={t("chat.systemPrompt")}
                        placeholder={t("settings.presets.contentPlaceholder")}
                        value={presetContent}
                        onChange={(e) => setPresetContent(e.currentTarget.value)}
                        minRows={3}
                        maxRows={10}
                        autosize
                    />
                    <Checkbox
                        label={t("settings.presets.useDefault")}
                        checked={presetIsDefault}
                        onChange={(e) => setPresetIsDefault(e.currentTarget.checked)}
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={closePresetModal}>
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={savePresetFromModal} disabled={!presetName.trim()}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            <ConfirmModal
                opened={deletingPresetId !== null}
                onClose={() => setDeletingPresetId(null)}
                onConfirm={() => {
                    if (deletingPresetId) {
                        deletePreset(deletingPresetId);
                        setDeletingPresetId(null);
                    }
                }}
                message={t("settings.presets.confirmDeletePreset")}
            />
        </Stack>
    );
}

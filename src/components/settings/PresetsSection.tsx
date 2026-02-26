import { useState } from "react";
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
                        Системные промпты (пресеты)
                    </Text>
                    <Button size="xs" variant="light" onClick={() => openPresetModal()}>
                        + Добавить пресет
                    </Button>
                </Group>
                {presets.length === 0 ? (
                    <Text size="xs" c="dimmed">
                        Нет пресетов. Добавьте пресет, чтобы задавать поведение модели для чатов.
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
                                        <Tooltip label="Редактировать">
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                onClick={() => openPresetModal(p)}
                                                aria-label="Редактировать"
                                            >
                                                <IconEdit size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label="Удалить">
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                color="red"
                                                onClick={() => setDeletingPresetId(p.id)}
                                                aria-label="Удалить"
                                            >
                                                <IconTrash size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={p.isDefault ? "По умолчанию" : "Сделать по умолчанию"}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                onClick={() => setPresetAsDefault(p.id)}
                                                aria-label={p.isDefault ? "По умолчанию" : "Сделать по умолчанию"}
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
                title={presetEditId ? "Редактировать пресет" : "Добавить пресет"}
                opened={presetModalOpen}
                onClose={closePresetModal}
                size="sm"
            >
                <Stack gap="sm">
                    <TextInput
                        label="Название"
                        placeholder="Название пресета"
                        value={presetName}
                        onChange={(e) => setPresetName(e.currentTarget.value)}
                    />
                    <Textarea
                        label="Системный промпт"
                        placeholder="Текст, задающий поведение модели..."
                        value={presetContent}
                        onChange={(e) => setPresetContent(e.currentTarget.value)}
                        minRows={3}
                        maxRows={10}
                        autosize
                    />
                    <Checkbox
                        label="Использовать по умолчанию"
                        checked={presetIsDefault}
                        onChange={(e) => setPresetIsDefault(e.currentTarget.checked)}
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={closePresetModal}>
                            Отмена
                        </Button>
                        <Button onClick={savePresetFromModal} disabled={!presetName.trim()}>
                            Сохранить
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
                message="Пресет будет удалён безвозвратно."
            />
        </Stack>
    );
}

import { useState } from "react";
import {
    ActionIcon,
    Badge,
    Button,
    Group,
    Modal,
    Paper,
    Select,
    Stack,
    Text,
    TextInput,
    Textarea,
} from "@mantine/core";
import { IconPhoto, IconPlus, IconPencil, IconTrash } from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import { useChatStore } from "../store/chatStore";
import type { Chat, ImageStyle } from "../types";

const SIZE_OPTIONS = [
    { value: "auto", label: "Auto" },
    { value: "1024x1024", label: "1024\u00d71024" },
    { value: "1024x1792", label: "1024\u00d71792" },
    { value: "1792x1024", label: "1792\u00d71024" },
    { value: "512x512", label: "512\u00d7512" },
    { value: "256x256", label: "256\u00d7256" },
];

const QUALITY_OPTIONS = [
    { value: "auto", label: "Auto" },
    { value: "standard", label: "Standard" },
    { value: "hd", label: "HD" },
    { value: "low", label: "Low" },
    { value: "medium", label: "Medium" },
    { value: "high", label: "High" },
];

const inputStyles = {
    input: {
        backgroundColor: "light-dark(var(--mantine-color-brand-1), var(--mantine-color-brand-8))",
        borderColor: "light-dark(var(--mantine-color-brand-3), var(--mantine-color-brand-7))",
        color: "var(--mantine-color-text)",
    },
};

interface Props {
    chatId: string;
    chat: Chat;
}

export function ImageConfigBar({ chatId, chat }: Props) {
    const { t } = useTranslation();
    const updateImageConfig = useChatStore((s) => s.updateImageConfig);
    const updateChatNegativePrompt = useChatStore((s) => s.updateChatNegativePrompt);
    const imageStyles = useChatStore((s) => s.imageStyles);
    const [stylesModalOpen, setStylesModalOpen] = useState(false);
    const [negPrompt, setNegPrompt] = useState(chat.negativePrompt ?? "");

    const styleOptions = imageStyles.map((s) => ({
        value: s.id,
        label: s.name,
    }));

    const saveNegativePrompt = () => {
        const val = negPrompt.trim() || null;
        if (val !== (chat.negativePrompt ?? null)) {
            updateChatNegativePrompt(chatId, val);
        }
    };

    return (
        <>
            <Paper p="xs" radius="md" mb={4} style={{ background: "light-dark(var(--mantine-color-brand-0), var(--mantine-color-brand-9))" }}>
                <Stack gap={4}>
                    <Group gap="xs" align="center" wrap="nowrap" w="100%">
                        <IconPhoto size={16} stroke={1.5} style={{ color: "var(--mantine-color-brand-5)", flexShrink: 0 }} />
                        <Select
                            size="xs"
                            placeholder={t("imageConfig.size")}
                            data={SIZE_OPTIONS}
                            value={chat.imageSize ?? null}
                            onChange={(val) => updateImageConfig(chatId, { imageSize: val })}
                            clearable
                            variant="filled"
                            style={{ flex: 1 }}
                            styles={inputStyles}
                        />
                        <Select
                            size="xs"
                            placeholder={t("imageConfig.quality")}
                            data={QUALITY_OPTIONS}
                            value={chat.imageQuality ?? null}
                            onChange={(val) => updateImageConfig(chatId, { imageQuality: val })}
                            clearable
                            variant="filled"
                            style={{ flex: 1 }}
                            styles={inputStyles}
                        />
                        <Select
                            size="xs"
                            placeholder={t("imageConfig.noStyle")}
                            data={styleOptions}
                            value={chat.imageStyle ?? null}
                            onChange={(val) => updateImageConfig(chatId, { imageStyle: val })}
                            clearable
                            variant="filled"
                            style={{ flex: 1 }}
                            styles={inputStyles}
                        />
                        <ActionIcon
                            size="xs"
                            variant="subtle"
                            onClick={() => setStylesModalOpen(true)}
                            title={t("imageConfig.manageStyles")}
                        >
                            <IconPlus size={14} />
                        </ActionIcon>
                    </Group>
                    <TextInput
                        size="xs"
                        placeholder={t("imageConfig.negativePrompt")}
                        value={negPrompt}
                        onChange={(e) => setNegPrompt(e.currentTarget.value)}
                        onBlur={saveNegativePrompt}
                        onKeyDown={(e) => { if (e.key === "Enter") saveNegativePrompt(); }}
                        variant="filled"
                        styles={inputStyles}
                    />
                </Stack>
            </Paper>
            <ImageStylesModal
                opened={stylesModalOpen}
                onClose={() => setStylesModalOpen(false)}
            />
        </>
    );
}

function ImageStylesModal({ opened, onClose }: { opened: boolean; onClose: () => void }) {
    const { t } = useTranslation();
    const imageStyles = useChatStore((s) => s.imageStyles);
    const createImageStyle = useChatStore((s) => s.createImageStyle);
    const updateImageStyle = useChatStore((s) => s.updateImageStyle);
    const deleteImageStyle = useChatStore((s) => s.deleteImageStyle);

    const [editingStyle, setEditingStyle] = useState<ImageStyle | null>(null);
    const [name, setName] = useState("");
    const [promptSuffix, setPromptSuffix] = useState("");

    const resetForm = () => {
        setEditingStyle(null);
        setName("");
        setPromptSuffix("");
    };

    const startEdit = (style: ImageStyle) => {
        setEditingStyle(style);
        setName(style.name);
        setPromptSuffix(style.promptSuffix);
    };

    const handleSave = async () => {
        if (!name.trim() || !promptSuffix.trim()) return;
        if (editingStyle) {
            await updateImageStyle(editingStyle.id, name.trim(), promptSuffix.trim());
        } else {
            await createImageStyle(name.trim(), promptSuffix.trim());
        }
        resetForm();
    };

    const handleDelete = async (id: string) => {
        await deleteImageStyle(id);
    };

    return (
        <Modal
            opened={opened}
            onClose={() => { onClose(); resetForm(); }}
            title={t("imageConfig.manageStyles")}
            size="md"
        >
            <Stack gap="sm">
                {imageStyles.map((style) => (
                    <Group key={style.id} justify="space-between" wrap="nowrap">
                        <div style={{ flex: 1, minWidth: 0 }}>
                            <Group gap="xs">
                                <Text size="sm" fw={500}>{style.name}</Text>
                                {style.isBuiltin && (
                                    <Badge size="xs" variant="light" color="gray">
                                        {t("imageConfig.builtinStyle")}
                                    </Badge>
                                )}
                            </Group>
                            <Text size="xs" c="dimmed" lineClamp={1}>{style.promptSuffix}</Text>
                        </div>
                        {!style.isBuiltin && (
                            <Group gap={4}>
                                <ActionIcon size="sm" variant="subtle" onClick={() => startEdit(style)}>
                                    <IconPencil size={14} />
                                </ActionIcon>
                                <ActionIcon size="sm" variant="subtle" color="red" onClick={() => handleDelete(style.id)}>
                                    <IconTrash size={14} />
                                </ActionIcon>
                            </Group>
                        )}
                    </Group>
                ))}

                <Paper p="xs" withBorder>
                    <Stack gap="xs">
                        <Text size="sm" fw={500}>
                            {editingStyle ? t("imageConfig.editStyle") : t("imageConfig.addStyle")}
                        </Text>
                        <TextInput
                            size="xs"
                            placeholder={t("imageConfig.styleName")}
                            value={name}
                            onChange={(e) => setName(e.currentTarget.value)}
                        />
                        <Textarea
                            size="xs"
                            placeholder={t("imageConfig.stylePromptHint")}
                            value={promptSuffix}
                            onChange={(e) => setPromptSuffix(e.currentTarget.value)}
                            minRows={2}
                        />
                        <Group gap="xs">
                            <Button size="xs" onClick={handleSave} disabled={!name.trim() || !promptSuffix.trim()}>
                                {editingStyle ? t("imageConfig.editStyle") : t("imageConfig.addStyle")}
                            </Button>
                            {editingStyle && (
                                <Button size="xs" variant="subtle" onClick={resetForm}>
                                    {t("common.cancel")}
                                </Button>
                            )}
                        </Group>
                    </Stack>
                </Paper>
            </Stack>
        </Modal>
    );
}

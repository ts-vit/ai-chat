import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
    Box,
    Button,
    Group,
    Modal,
    Stack,
    Text,
    TextInput,
} from "@mantine/core";
import { ColorSwatch } from "@mantine/core";
import { FOLDER_COLORS } from "../constants/folderColors";
import { useChatStore } from "../store/chatStore";

interface CreateFolderModalProps {
    opened: boolean;
    onClose: () => void;
}

export function CreateFolderModal({ opened, onClose }: CreateFolderModalProps) {
    const { t } = useTranslation();
    const createFolder = useChatStore((s) => s.createFolder);

    const [name, setName] = useState("");
    const [nameError, setNameError] = useState<string | null>(null);
    const [color, setColor] = useState<string | null>(null);

    const resetFields = useCallback(() => {
        setName("");
        setNameError(null);
        setColor(null);
    }, []);

    const handleClose = useCallback(() => {
        resetFields();
        onClose();
    }, [resetFields, onClose]);

    const handleSubmit = useCallback(() => {
        const trimmed = name.trim();
        if (!trimmed) {
            setNameError(t("common.fieldRequired"));
            return;
        }
        setNameError(null);
        createFolder(trimmed, color);
        resetFields();
        onClose();
    }, [name, color, createFolder, resetFields, onClose, t]);

    return (
        <Modal
            size="xs"
            title={t("sidebar.newFolder")}
            opened={opened}
            onClose={handleClose}
        >
            <Stack gap="lg">
                <TextInput
                    placeholder={t("sidebar.folderName")}
                    value={name}
                    onChange={(e) => {
                        setName(e.currentTarget.value);
                        setNameError(null);
                    }}
                    error={nameError}
                    onKeyDown={(e) => {
                        if (e.key === "Enter") handleSubmit();
                    }}
                    autoFocus
                />
                <Box>
                    <Text size="sm" fw={500} mb="xs" component="label">
                        {t("sidebar.folderColor")}
                    </Text>
                    <Group gap={4}>
                        {FOLDER_COLORS.map((c) => (
                            <ColorSwatch
                                key={c}
                                color={`var(--mantine-color-${c}-5)`}
                                size={16}
                                onClick={() => setColor(c)}
                                style={{
                                    cursor: "pointer",
                                    border:
                                        color === c
                                            ? "2px solid var(--mantine-color-default-border)"
                                            : undefined,
                                }}
                            />
                        ))}
                    </Group>
                </Box>
                <Group justify="flex-end" gap="sm">
                    <Button variant="subtle" onClick={handleClose}>
                        {t("common.cancel")}
                    </Button>
                    <Button onClick={handleSubmit}>
                        {t("common.create")}
                    </Button>
                </Group>
            </Stack>
        </Modal>
    );
}

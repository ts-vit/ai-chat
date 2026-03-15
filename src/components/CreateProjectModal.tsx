import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
    Button,
    Group,
    Modal,
    Stack,
    TextInput,
    Textarea,
} from "@mantine/core";
import { useChatStore } from "../store/chatStore";

interface CreateProjectModalProps {
    opened: boolean;
    onClose: () => void;
}

export function CreateProjectModal({ opened, onClose }: CreateProjectModalProps) {
    const { t } = useTranslation();
    const createProject = useChatStore((s) => s.createProject);

    const [name, setName] = useState("");
    const [goal, setGoal] = useState("");
    const [nameError, setNameError] = useState<string | null>(null);

    const resetFields = useCallback(() => {
        setName("");
        setGoal("");
        setNameError(null);
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
        createProject(trimmed, goal.trim());
        resetFields();
        onClose();
    }, [name, goal, createProject, resetFields, onClose, t]);

    return (
        <Modal
            size="sm"
            title={t("project.new")}
            opened={opened}
            onClose={handleClose}
        >
            <Stack gap="lg">
                <TextInput
                    label={t("project.name")}
                    placeholder={t("project.name")}
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
                <Textarea
                    label={t("project.goal")}
                    placeholder={t("project.goalPlaceholder")}
                    value={goal}
                    onChange={(e) => setGoal(e.currentTarget.value)}
                    minRows={2}
                    maxRows={4}
                    autosize
                />
                <Group justify="flex-end" gap="sm">
                    <Button variant="subtle" onClick={handleClose}>
                        {t("common.cancel")}
                    </Button>
                    <Button onClick={handleSubmit}>
                        {t("project.create")}
                    </Button>
                </Group>
            </Stack>
        </Modal>
    );
}

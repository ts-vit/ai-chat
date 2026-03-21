import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Group, Modal, Select, Stack, TextInput, Textarea } from "@mantine/core";
import { useChatStore } from "../store/chatStore";

interface CreateKnowledgeBaseModalProps {
    opened: boolean;
    onClose: () => void;
}

export function CreateKnowledgeBaseModal({ opened, onClose }: CreateKnowledgeBaseModalProps) {
    const { t } = useTranslation();
    const createKnowledgeBase = useChatStore((s) => s.createKnowledgeBase);

    const [name, setName] = useState("");
    const [description, setDescription] = useState("");
    const [embeddingModel, setEmbeddingModel] = useState("e5-small");
    const [nameError, setNameError] = useState<string | null>(null);

    const resetFields = useCallback(() => {
        setName("");
        setDescription("");
        setEmbeddingModel("e5-small");
        setNameError(null);
    }, []);

    const handleClose = useCallback(() => {
        resetFields();
        onClose();
    }, [resetFields, onClose]);

    const handleSubmit = useCallback(async () => {
        const trimmed = name.trim();
        if (!trimmed) {
            setNameError(t("common.fieldRequired"));
            return;
        }
        setNameError(null);
        await createKnowledgeBase(trimmed, description.trim(), embeddingModel);
        resetFields();
        onClose();
    }, [name, description, embeddingModel, createKnowledgeBase, resetFields, onClose, t]);

    return (
        <Modal size="sm" title={t("kb.create")} opened={opened} onClose={handleClose}>
            <Stack gap="lg">
                <TextInput
                    label={t("kb.name")}
                    placeholder={t("kb.namePlaceholder")}
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
                    label={t("kb.description")}
                    placeholder={t("kb.descriptionPlaceholder")}
                    value={description}
                    onChange={(e) => setDescription(e.currentTarget.value)}
                    minRows={2}
                    maxRows={4}
                    autosize
                />
                <Select
                    label={t("kb.embeddingModelSelect")}
                    value={embeddingModel}
                    onChange={(v) => setEmbeddingModel(v ?? "e5-small")}
                    data={[
                        { value: "e5-small", label: t("kb.embeddingE5Small") },
                        { value: "openai", label: t("kb.embeddingOpenai") },
                        { value: "gemini", label: t("kb.embeddingGemini") },
                    ]}
                />
                <Group justify="flex-end" gap="sm">
                    <Button variant="subtle" onClick={handleClose}>{t("common.cancel")}</Button>
                    <Button onClick={handleSubmit}>{t("common.create")}</Button>
                </Group>
            </Stack>
        </Modal>
    );
}

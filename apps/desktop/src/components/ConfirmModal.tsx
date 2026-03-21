import { useTranslation } from "react-i18next";
import { Button, Group, Modal, Text } from "@mantine/core";

interface ConfirmModalProps {
    opened: boolean;
    onClose: () => void;
    onConfirm: () => void;
    title?: string;
    message: string;
    confirmLabel?: string;
}

export function ConfirmModal({
    opened,
    onClose,
    onConfirm,
    title,
    message,
    confirmLabel,
}: ConfirmModalProps) {
    const { t } = useTranslation();
    const handleConfirm = () => {
        onConfirm();
        onClose();
    };

    return (
        <Modal opened={opened} onClose={onClose} title={title ?? t("confirm.defaultTitle")} size="sm">
            <Text>{message}</Text>
            <Group mt="md" justify="flex-end" gap="sm">
                <Button variant="subtle" onClick={onClose}>
                    {t("common.cancel")}
                </Button>
                <Button color="red" onClick={handleConfirm}>
                    {confirmLabel ?? t("confirm.delete")}
                </Button>
            </Group>
        </Modal>
    );
}

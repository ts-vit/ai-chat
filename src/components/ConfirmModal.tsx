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
    title = "Подтверждение",
    message,
    confirmLabel = "Удалить",
}: ConfirmModalProps) {
    const handleConfirm = () => {
        onConfirm();
        onClose();
    };

    return (
        <Modal opened={opened} onClose={onClose} title={title} size="sm">
            <Text>{message}</Text>
            <Group mt="md" justify="flex-end" gap="sm">
                <Button variant="subtle" onClick={onClose}>
                    Отмена
                </Button>
                <Button color="red" onClick={handleConfirm}>
                    {confirmLabel}
                </Button>
            </Group>
        </Modal>
    );
}

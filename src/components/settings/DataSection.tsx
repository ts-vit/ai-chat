import { Button, Divider, Stack, Text } from "@mantine/core";
import { invoke } from "@tauri-apps/api/core";
import { useChatStore } from "../../store/chatStore";
import { notify } from "../../utils/notify";
import type { ImportResult } from "../../types";

export function DataSection() {
    const loadChats = useChatStore((s) => s.loadChats);

    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Резервное копирование
                </Text>
                <Button
                    variant="light"
                    size="sm"
                    onClick={async () => {
                        try {
                            await invoke("export_all_chats");
                            notify.success("Чаты экспортированы");
                        } catch (e) {
                            notify.error(String(e));
                        }
                    }}
                >
                    Экспорт всех чатов
                </Button>
                <Text size="xs" c="dimmed">
                    Скачать все чаты в ZIP-архив для резервной копии
                </Text>
            </Stack>

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Импорт чатов
                </Text>
                <Button
                    variant="light"
                    size="sm"
                    onClick={async () => {
                        try {
                            const result = await invoke<ImportResult>("import_chats");
                            notify.success(
                                `Импортировано чатов: ${result.chatsImported}, сообщений: ${result.messagesImported}, вложений: ${result.attachmentsImported}`
                            );
                            await loadChats();
                        } catch (e) {
                            notify.error(String(e));
                        }
                    }}
                >
                    Импорт чатов
                </Button>
                <Text size="xs" c="dimmed">
                    Восстановить чаты из ранее экспортированного ZIP-архива
                </Text>
            </Stack>

            <Divider />

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Очистка данных
                </Text>
                <Button variant="light" size="sm" color="red" disabled>
                    Удалить все чаты
                </Button>
                <Text size="xs" c="dimmed">
                    Безвозвратно удалить все чаты и сообщения из приложения
                </Text>
            </Stack>
        </Stack>
    );
}

import { Button, Divider, Stack, Text } from "@mantine/core";

export function DataSection() {
    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Экспорт чатов
                </Text>
                <Button variant="light" size="sm" disabled>
                    Экспорт в JSON
                </Button>
                <Text size="xs" c="dimmed">
                    Скачать все чаты в виде JSON-файла для резервной копии
                </Text>
            </Stack>

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Импорт чатов
                </Text>
                <Button variant="light" size="sm" disabled>
                    Импорт из JSON
                </Button>
                <Text size="xs" c="dimmed">
                    Восстановить чаты из ранее экспортированного JSON-файла
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

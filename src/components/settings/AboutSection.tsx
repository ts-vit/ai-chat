import { Anchor, Stack, Text } from "@mantine/core";

const APP_VERSION = "0.1.0";

export function AboutSection() {
    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="lg" fw={600}>
                    AI Chat
                </Text>
                <Text size="sm" c="dimmed">
                    Версия {APP_VERSION}
                </Text>
            </Stack>
            <Text size="sm" c="dimmed">
                Tauri 2 + React + Rust
            </Text>
            <Text size="sm">
                Десктопный AI-чат клиент с поддержкой OpenRouter, Ollama и OpenAI-совместимых провайдеров.
            </Text>
            <Anchor href="#" size="sm">
                GitHub
            </Anchor>
        </Stack>
    );
}

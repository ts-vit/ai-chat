import { Stack, Text, TextInput } from "@mantine/core";
import { OllamaSection } from "../OllamaSection";

export interface OllamaModelsSectionProps {
    ollamaUrl: string;
    onOllamaUrlChange: (url: string) => void;
    ollamaEnabledModels: string[];
    onOllamaEnabledModelsChange: (ids: string[]) => void;
}

export function OllamaModelsSection({
    ollamaUrl,
    onOllamaUrlChange,
    ollamaEnabledModels,
    onOllamaEnabledModelsChange,
}: OllamaModelsSectionProps) {
    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Ollama URL
                </Text>
                <TextInput
                    placeholder="http://localhost:11434/v1"
                    value={ollamaUrl}
                    onChange={(e) => onOllamaUrlChange(e.currentTarget.value)}
                />
                <Text size="xs" c="dimmed">
                    Изменяйте только если Ollama запущена на другом порту
                </Text>
            </Stack>
            <OllamaSection
                ollamaUrl={ollamaUrl}
                onOllamaUrlChange={onOllamaUrlChange}
                ollamaEnabledModels={ollamaEnabledModels}
                onOllamaEnabledModelsChange={onOllamaEnabledModelsChange}
            />
        </Stack>
    );
}

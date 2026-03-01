import { useTranslation } from "react-i18next";
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
    const { t } = useTranslation();
    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("settings.ollama.url")}
                </Text>
                <TextInput
                    placeholder={t("ollama.serverUrlPlaceholder")}
                    value={ollamaUrl}
                    onChange={(e) => onOllamaUrlChange(e.currentTarget.value)}
                />
                <Text size="xs" c="dimmed">
                    {t("settings.ollama.urlHint")}
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

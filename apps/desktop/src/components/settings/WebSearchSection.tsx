import { useTranslation } from "react-i18next";
import { PasswordInput, Select, Stack, Text, Title } from "@mantine/core";

interface Props {
    webSearchProvider: string;
    onWebSearchProviderChange: (value: string) => void;
    tavilyApiKey: string;
    onTavilyApiKeyChange: (value: string) => void;
    braveApiKey: string;
    onBraveApiKeyChange: (value: string) => void;
}

export function WebSearchSection({
    webSearchProvider,
    onWebSearchProviderChange,
    tavilyApiKey,
    onTavilyApiKeyChange,
    braveApiKey,
    onBraveApiKeyChange,
}: Props) {
    const { t } = useTranslation();

    return (
        <Stack gap="lg">
            <Title order={4}>{t("settings.webSearch.title")}</Title>

            <Select
                label={t("settings.webSearch.provider")}
                value={webSearchProvider || ""}
                onChange={(value) => onWebSearchProviderChange(value ?? "")}
                data={[
                    { value: "", label: "—" },
                    { value: "tavily", label: "Tavily" },
                    { value: "brave", label: "Brave Search" },
                    { value: "uni", label: "UNI Search" },
                ]}
            />

            {webSearchProvider === "tavily" && (
                <Stack gap="sm">
                    <PasswordInput
                        label={t("settings.webSearch.tavilyApiKey")}
                        value={tavilyApiKey}
                        onChange={(e) => onTavilyApiKeyChange(e.currentTarget.value)}
                    />
                    <Text size="xs" c="dimmed">
                        {t("settings.webSearch.tavilyHint")}
                    </Text>
                </Stack>
            )}

            {webSearchProvider === "brave" && (
                <Stack gap="sm">
                    <PasswordInput
                        label={t("settings.webSearch.braveApiKey")}
                        value={braveApiKey}
                        onChange={(e) => onBraveApiKeyChange(e.currentTarget.value)}
                    />
                    <Text size="xs" c="dimmed">
                        {t("settings.webSearch.braveHint")}
                    </Text>
                </Stack>
            )}

            {webSearchProvider === "uni" && (
                <Text size="xs" c="dimmed">
                    {t("settings.webSearch.uniDescription")}
                </Text>
            )}
        </Stack>
    );
}

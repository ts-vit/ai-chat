import { useTranslation } from "react-i18next";
import { Anchor, Stack, Text } from "@mantine/core";

const APP_VERSION = "0.1.0";

export function AboutSection() {
    const { t } = useTranslation();
    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="lg" fw={600}>
                    {t("settings.about.appName")}
                </Text>
                <Text size="sm" c="dimmed">
                    {t("settings.about.version", { version: APP_VERSION })}
                </Text>
            </Stack>
            <Text size="sm" c="dimmed">
                {t("settings.about.stack")}
            </Text>
            <Text size="sm">
                {t("settings.about.description")}
            </Text>
            <Anchor href="#" size="sm">
                {t("settings.about.github")}
            </Anchor>
        </Stack>
    );
}

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Anchor, Button, Stack, Text } from "@mantine/core";
import { getVersion } from "@tauri-apps/api/app";
import { notify } from "../../utils/notify";

import logo from "../../assets/sp-logo.png";

export function AboutSection() {
    const { t } = useTranslation();
    const [appVersion, setAppVersion] = useState<string>("");

    useEffect(() => {
        getVersion()
            .then(setAppVersion)
            .catch(() => setAppVersion("0.1.0"));
    }, []);

    return (
        <Stack gap="lg" align="center">
            <img
                src={logo}
                alt="Logo"
                style={{ height: 64, objectFit: "contain" }}
            />
            <Stack gap="xs" align="center">
                <Text size="lg" fw={600}>
                    {t("settings.about.appName")}
                </Text>
                {appVersion && (
                    <Text size="sm" c="dimmed">
                        {t("settings.about.version", { version: appVersion })}
                    </Text>
                )}
            </Stack>
            <Text size="xs" c="dimmed">
                {t("settings.about.stack")}
            </Text>
            <Text size="sm" c="dimmed">
                {t("settings.about.description")}
            </Text>
            <Stack gap="xs" align="center">
                <Anchor href="https://github.com" target="_blank" rel="noopener noreferrer" size="sm">
                    {t("settings.about.github")}
                </Anchor>
                <Anchor href="#" size="sm">
                    {t("settings.about.website")}
                </Anchor>
            </Stack>
            <Button
                variant="light"
                size="sm"
                onClick={() => notify.info(t("settings.about.noUpdates"))}
            >
                {t("settings.about.checkUpdates")}
            </Button>
            <Text size="xs" c="dimmed">
                {t("settings.about.license")}
            </Text>
        </Stack>
    );
}

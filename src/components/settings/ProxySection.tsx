import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Button,
    Group,
    NumberInput,
    PasswordInput,
    Select,
    Stack,
    Switch,
    Text,
    TextInput,
    Title,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconPlugConnected } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
    proxyEnabled: boolean;
    onProxyEnabledChange: (value: boolean) => void;
    proxyType: string;
    onProxyTypeChange: (value: string) => void;
    proxyHost: string;
    onProxyHostChange: (value: string) => void;
    proxyPort: number | undefined;
    onProxyPortChange: (value: number | undefined) => void;
    proxyUsername: string;
    onProxyUsernameChange: (value: string) => void;
    proxyPassword: string;
    onProxyPasswordChange: (value: string) => void;
}

export function ProxySection({
    proxyEnabled,
    onProxyEnabledChange,
    proxyType,
    onProxyTypeChange,
    proxyHost,
    onProxyHostChange,
    proxyPort,
    onProxyPortChange,
    proxyUsername,
    onProxyUsernameChange,
    proxyPassword,
    onProxyPasswordChange,
}: Props) {
    const { t } = useTranslation();
    const [testing, setTesting] = useState(false);

    const handleTestProxy = async () => {
        setTesting(true);
        try {
            const ip = await invoke<string>("test_proxy", {
                proxyType: proxyType || "http",
                proxyHost,
                proxyPort: proxyPort || 8080,
                proxyUsername: proxyUsername || null,
                proxyPassword: proxyPassword || null,
            });
            notifications.show({
                message: t("settings.proxy.testSuccess", { ip }),
                color: "green",
                autoClose: 3000,
            });
        } catch (e: unknown) {
            notifications.show({
                message: t("settings.proxy.testError") + ": " + String(e),
                color: "red",
                autoClose: 5000,
            });
        } finally {
            setTesting(false);
        }
    };

    return (
        <Stack gap="lg">
            <Title order={4}>{t("settings.proxy.title")}</Title>

            <Switch
                label={t("settings.proxy.enabled")}
                description={t("settings.proxy.enabledDescription")}
                checked={proxyEnabled}
                onChange={(e) => onProxyEnabledChange(e.currentTarget.checked)}
            />

            {proxyEnabled && (
                <Stack gap="sm">
                    <Select
                        label={t("settings.proxy.type")}
                        value={proxyType || "http"}
                        onChange={(value) => onProxyTypeChange(value ?? "http")}
                        data={[
                            { value: "http", label: "HTTP" },
                            { value: "https", label: "HTTPS" },
                            { value: "socks5", label: "SOCKS5" },
                        ]}
                    />

                    <Group align="flex-end">
                        <TextInput
                            style={{ flex: 1 }}
                            label={t("settings.proxy.host")}
                            placeholder={t("settings.proxy.hostPlaceholder")}
                            value={proxyHost}
                            onChange={(e) => onProxyHostChange(e.currentTarget.value)}
                        />
                        <NumberInput
                            w={120}
                            label={t("settings.proxy.port")}
                            placeholder={t("settings.proxy.portPlaceholder")}
                            value={proxyPort ?? ""}
                            onChange={(value) =>
                                onProxyPortChange(typeof value === "number" ? value : undefined)
                            }
                            min={1}
                            max={65535}
                        />
                    </Group>

                    <Group grow>
                        <TextInput
                            label={t("settings.proxy.username")}
                            placeholder={t("settings.proxy.usernamePlaceholder")}
                            value={proxyUsername}
                            onChange={(e) => onProxyUsernameChange(e.currentTarget.value)}
                        />
                        <PasswordInput
                            label={t("settings.proxy.password")}
                            placeholder={t("settings.proxy.passwordPlaceholder")}
                            value={proxyPassword}
                            onChange={(e) => onProxyPasswordChange(e.currentTarget.value)}
                        />
                    </Group>

                    <Text size="xs" c="dimmed">
                        {t("settings.proxy.enabledDescription")}
                    </Text>

                    <Button
                        variant="light"
                        leftSection={<IconPlugConnected size={16} stroke={1.5} />}
                        loading={testing}
                        onClick={handleTestProxy}
                        w="fit-content"
                    >
                        {t("settings.proxy.testConnection")}
                    </Button>
                </Stack>
            )}
        </Stack>
    );
}

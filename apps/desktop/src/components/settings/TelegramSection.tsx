import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
    Badge,
    Button,
    Group,
    PasswordInput,
    Stack,
    Switch,
    Text,
    TextInput,
    Title,
} from "@mantine/core";
import { IconBrandTelegram } from "@tabler/icons-react";
import { useChatStore } from "../../store/chatStore";
import { notify } from "../../utils/notify";
import type { TelegramAuthRequest } from "../../types";

interface Props {
    telegramBotToken: string;
    onTelegramBotTokenChange: (value: string) => void;
    telegramEnabled: boolean;
    onTelegramEnabledChange: (value: boolean) => void;
    telegramAutoStart: boolean;
    onTelegramAutoStartChange: (value: boolean) => void;
    telegramModel: string;
    onTelegramModelChange: (value: string) => void;
}

export function TelegramSection({
    telegramBotToken,
    onTelegramBotTokenChange,
    telegramEnabled,
    onTelegramEnabledChange,
    telegramAutoStart,
    onTelegramAutoStartChange,
    telegramModel,
    onTelegramModelChange,
}: Props) {
    const { t } = useTranslation();
    const {
        telegramStatus,
        startTelegramBot,
        stopTelegramBot,
        getTelegramStatus,
        revokeTelegramUser,
        authorizeTelegramUser,
    } = useChatStore();
    const [validatedUsername, setValidatedUsername] = useState<string | null>(null);
    const [validating, setValidating] = useState(false);
    const [pendingAuth, setPendingAuth] = useState<TelegramAuthRequest | null>(null);

    // Refresh status on mount and listen for events
    useEffect(() => {
        getTelegramStatus();

        const unlistenAuth = listen<TelegramAuthRequest>("telegram-auth-request", (event) => {
            setPendingAuth(event.payload);
            const name = event.payload.username
                ? `${event.payload.firstName} (@${event.payload.username})`
                : event.payload.firstName;
            notify.warning(t("telegram.authRequest", { name }));
        });

        const unlistenStatus = listen("telegram-status-changed", () => {
            getTelegramStatus();
        });

        return () => {
            unlistenAuth.then((fn) => fn());
            unlistenStatus.then((fn) => fn());
        };
    }, []);

    const handleValidate = async () => {
        if (!telegramBotToken.trim()) return;
        setValidating(true);
        try {
            const info = await useChatStore.getState().validateTelegramToken(telegramBotToken);
            setValidatedUsername(info.username);
            notify.success(t("telegram.validToken", { username: info.username }));
        } catch (e) {
            setValidatedUsername(null);
            notify.error(t("telegram.invalidToken") + ": " + String(e));
        } finally {
            setValidating(false);
        }
    };

    const handleToggleBot = async () => {
        if (telegramStatus.running) {
            await stopTelegramBot();
        } else {
            await startTelegramBot();
        }
    };

    const handleApproveAuth = async () => {
        if (!pendingAuth) return;
        await authorizeTelegramUser(
            pendingAuth.userId,
            pendingAuth.firstName,
            pendingAuth.lastName,
            pendingAuth.username
        );
        notify.success(t("telegram.authApproved"));
        setPendingAuth(null);
    };

    return (
        <Stack gap="lg">
            <Group gap="xs">
                <IconBrandTelegram size={22} stroke={1.5} />
                <Title order={4}>{t("telegram.title")}</Title>
            </Group>

            <PasswordInput
                label={t("telegram.botToken")}
                placeholder={t("telegram.botTokenPlaceholder")}
                value={telegramBotToken}
                onChange={(e) => {
                    onTelegramBotTokenChange(e.currentTarget.value);
                    setValidatedUsername(null);
                }}
            />

            <Group gap="sm">
                <Button
                    size="xs"
                    variant="light"
                    onClick={handleValidate}
                    loading={validating}
                    disabled={!telegramBotToken.trim()}
                >
                    {t("telegram.validate")}
                </Button>
                {validatedUsername && (
                    <Text size="sm" c="green">
                        ✓ @{validatedUsername}
                    </Text>
                )}
            </Group>

            <Group gap="sm">
                <Text size="sm" fw={500}>{t("telegram.status")}:</Text>
                <Badge
                    color={telegramStatus.running ? "green" : "gray"}
                    variant="dot"
                >
                    {telegramStatus.running
                        ? t("telegram.connected") + (telegramStatus.botUsername ? ` (@${telegramStatus.botUsername})` : "")
                        : t("telegram.disconnected")
                    }
                </Badge>
            </Group>

            <Button
                size="xs"
                variant={telegramStatus.running ? "outline" : "filled"}
                color={telegramStatus.running ? "red" : "brand"}
                onClick={handleToggleBot}
                disabled={!telegramBotToken.trim()}
                w="fit-content"
            >
                {telegramStatus.running ? t("telegram.disconnected") : t("telegram.connected")}
            </Button>

            {pendingAuth && (
                <Group gap="sm" p="sm" style={{ border: "1px solid var(--mantine-color-yellow-5)", borderRadius: "var(--mantine-radius-md)" }}>
                    <Text size="sm">
                        {t("telegram.authRequest", {
                            name: pendingAuth.username
                                ? `${pendingAuth.firstName} (@${pendingAuth.username})`
                                : pendingAuth.firstName,
                        })}
                    </Text>
                    <Button size="xs" color="green" onClick={handleApproveAuth}>
                        {t("telegram.authApprove")}
                    </Button>
                    <Button size="xs" variant="subtle" color="gray" onClick={() => setPendingAuth(null)}>
                        {t("telegram.authDeny")}
                    </Button>
                </Group>
            )}

            <Group gap="sm">
                <Text size="sm" fw={500}>{t("telegram.authorizedUser")}:</Text>
                <Text size="sm" c={telegramStatus.authorizedUser ? undefined : "dimmed"}>
                    {telegramStatus.authorizedUser || t("telegram.notBound")}
                </Text>
                {telegramStatus.authorizedUser && (
                    <Button size="xs" variant="subtle" color="red" onClick={revokeTelegramUser}>
                        {t("telegram.revoke")}
                    </Button>
                )}
            </Group>

            <TextInput
                label={t("telegram.model")}
                placeholder={t("telegram.modelPlaceholder")}
                description={t("telegram.modelHint")}
                value={telegramModel}
                onChange={(e) => onTelegramModelChange(e.currentTarget.value)}
            />

            <Switch
                label={t("telegram.enable")}
                checked={telegramEnabled}
                onChange={(e) => onTelegramEnabledChange(e.currentTarget.checked)}
            />

            <Switch
                label={t("telegram.autoStart")}
                checked={telegramAutoStart}
                onChange={(e) => onTelegramAutoStartChange(e.currentTarget.checked)}
            />

            <Text size="xs" c="dimmed">
                {t("telegram.howToCreate")}
            </Text>
        </Stack>
    );
}

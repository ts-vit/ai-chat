import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Alert,
    Badge,
    Button,
    Group,
    NumberInput,
    PasswordInput,
    SegmentedControl,
    Stack,
    Switch,
    Text,
    TextInput,
    Title,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import {
    IconAlertTriangle,
    IconPlugConnected,
    IconPlugConnectedX,
    IconFolder,
} from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

interface SshTunnelStatus {
    connected: boolean;
    localPort: number | null;
    remoteHost: string | null;
}

interface Props {
    sshHost: string;
    onSshHostChange: (value: string) => void;
    sshPort: number | undefined;
    onSshPortChange: (value: number | undefined) => void;
    sshUsername: string;
    onSshUsernameChange: (value: string) => void;
    sshAuthType: string;
    onSshAuthTypeChange: (value: string) => void;
    sshPassword: string;
    onSshPasswordChange: (value: string) => void;
    sshKeyPath: string;
    onSshKeyPathChange: (value: string) => void;
    sshAutoConnect: boolean;
    onSshAutoConnectChange: (value: boolean) => void;
}

export function SshTunnelSection({
    sshHost,
    onSshHostChange,
    sshPort,
    onSshPortChange,
    sshUsername,
    onSshUsernameChange,
    sshAuthType,
    onSshAuthTypeChange,
    sshPassword,
    onSshPasswordChange,
    sshKeyPath,
    onSshKeyPathChange,
    sshAutoConnect,
    onSshAutoConnectChange,
}: Props) {
    const { t } = useTranslation();
    const [connecting, setConnecting] = useState(false);
    const [tunnelStatus, setTunnelStatus] = useState<SshTunnelStatus>({
        connected: false,
        localPort: null,
        remoteHost: null,
    });
    const [reconnecting, setReconnecting] = useState(false);
    const [reconnectAttempt, setReconnectAttempt] = useState(0);
    const [hostKeyMismatch, setHostKeyMismatch] = useState(false);

    useEffect(() => {
        invoke<SshTunnelStatus>("ssh_tunnel_status").then(setTunnelStatus).catch(() => {});

        const unlistenConnected = listen<{ host: string; port: number }>(
            "ssh-tunnel-connected",
            (event) => {
                setTunnelStatus({
                    connected: true,
                    localPort: event.payload.port,
                    remoteHost: event.payload.host,
                });
            }
        );
        const unlistenDisconnected = listen("ssh-tunnel-disconnected", () => {
            setTunnelStatus({ connected: false, localPort: null, remoteHost: null });
            setReconnecting(false);
            setReconnectAttempt(0);
        });

        const unlistenReconnecting = listen("ssh-tunnel-reconnecting", () => {
            setReconnecting(true);
            setReconnectAttempt(0);
        });

        const unlistenReconnectAttempt = listen<{ attempt: number; maxAttempts: number }>(
            "ssh-tunnel-reconnect-attempt",
            (event) => {
                setReconnectAttempt(event.payload.attempt);
            }
        );

        const unlistenReconnected = listen<{ port: number }>(
            "ssh-tunnel-reconnected",
            (event) => {
                setReconnecting(false);
                setReconnectAttempt(0);
                setTunnelStatus((prev) => ({
                    connected: true,
                    localPort: event.payload.port,
                    remoteHost: prev.remoteHost,
                }));
                notifications.show({
                    message: t("settings.vpn.ssh.reconnected"),
                    color: "green",
                    autoClose: 3000,
                });
            }
        );

        const unlistenReconnectFailed = listen("ssh-tunnel-reconnect-failed", () => {
            setReconnecting(false);
            setReconnectAttempt(0);
            notifications.show({
                message: t("settings.vpn.ssh.reconnectFailed"),
                color: "red",
                autoClose: 10000,
            });
        });

        const unlistenHostKeyChanged = listen<{ host: string; port: number }>(
            "ssh-host-key-changed",
            (event) => {
                setHostKeyMismatch(true);
                notifications.show({
                    message: t("settings.vpn.ssh.hostKeyChangedMessage", {
                        host: event.payload.host,
                        port: event.payload.port,
                    }),
                    color: "red",
                    autoClose: false,
                });
            }
        );

        return () => {
            unlistenConnected.then((f) => f());
            unlistenDisconnected.then((f) => f());
            unlistenReconnecting.then((f) => f());
            unlistenReconnectAttempt.then((f) => f());
            unlistenReconnected.then((f) => f());
            unlistenReconnectFailed.then((f) => f());
            unlistenHostKeyChanged.then((f) => f());
        };
    }, []);

    const handleConnect = async () => {
        setConnecting(true);
        try {
            let privateKey: string | null = null;
            if (sshAuthType === "key" && sshKeyPath) {
                privateKey = sshKeyPath;
            }
            const port = await invoke<number>("ssh_tunnel_connect", {
                host: sshHost,
                port: sshPort || 22,
                username: sshUsername,
                authType: sshAuthType || "password",
                password: sshAuthType === "key" ? null : sshPassword || null,
                privateKey: sshAuthType === "key" ? privateKey : null,
            });
            notifications.show({
                message: t("settings.vpn.ssh.connectSuccess", { port }),
                color: "green",
                autoClose: 3000,
            });
        } catch (e: unknown) {
            notifications.show({
                message: t("settings.vpn.ssh.connectError") + ": " + String(e),
                color: "red",
                autoClose: 5000,
            });
        } finally {
            setConnecting(false);
        }
    };

    const handleDisconnect = async () => {
        try {
            await invoke("ssh_tunnel_disconnect");
            notifications.show({
                message: t("settings.vpn.ssh.disconnectSuccess"),
                color: "green",
                autoClose: 3000,
            });
        } catch (e: unknown) {
            notifications.show({
                message: String(e),
                color: "red",
                autoClose: 5000,
            });
        }
    };

    const handleResetHostKey = async () => {
        try {
            await invoke("ssh_remove_known_host", {
                host: sshHost,
                port: sshPort || 22,
            });
            setHostKeyMismatch(false);
            notifications.show({
                message: t("settings.vpn.ssh.hostKeyReset"),
                color: "green",
                autoClose: 3000,
            });
        } catch (e: unknown) {
            notifications.show({
                message: String(e),
                color: "red",
                autoClose: 5000,
            });
        }
    };

    const handleBrowseKey = async () => {
        const result = await open({
            multiple: false,
            filters: [{ name: "SSH Key", extensions: ["pem", "key", "ppk", "*"] }],
        });
        if (result) {
            onSshKeyPathChange(result as string);
        }
    };

    return (
        <Stack gap="lg">
            <Title order={4}>{t("settings.vpn.title")}</Title>

            <Stack gap="sm">
                <Group align="flex-end">
                    <TextInput
                        style={{ flex: 1 }}
                        label={t("settings.vpn.ssh.host")}
                        placeholder={t("settings.vpn.ssh.hostPlaceholder")}
                        value={sshHost}
                        onChange={(e) => onSshHostChange(e.currentTarget.value)}
                    />
                    <NumberInput
                        w={100}
                        label={t("settings.vpn.ssh.port")}
                        value={sshPort ?? 22}
                        onChange={(value) =>
                            onSshPortChange(typeof value === "number" ? value : undefined)
                        }
                        min={1}
                        max={65535}
                    />
                </Group>

                <TextInput
                    label={t("settings.vpn.ssh.username")}
                    placeholder={t("settings.vpn.ssh.usernamePlaceholder")}
                    value={sshUsername}
                    onChange={(e) => onSshUsernameChange(e.currentTarget.value)}
                />

                <SegmentedControl
                    value={sshAuthType || "password"}
                    onChange={onSshAuthTypeChange}
                    data={[
                        { value: "password", label: t("settings.vpn.ssh.authPassword") },
                        { value: "key", label: t("settings.vpn.ssh.authKey") },
                    ]}
                    size="xs"
                />

                {(sshAuthType || "password") === "password" ? (
                    <PasswordInput
                        label={t("settings.vpn.ssh.password")}
                        placeholder={t("settings.vpn.ssh.passwordPlaceholder")}
                        value={sshPassword}
                        onChange={(e) => onSshPasswordChange(e.currentTarget.value)}
                    />
                ) : (
                    <Group align="flex-end">
                        <TextInput
                            style={{ flex: 1 }}
                            label={t("settings.vpn.ssh.keyPath")}
                            placeholder={t("settings.vpn.ssh.keyPathPlaceholder")}
                            value={sshKeyPath}
                            onChange={(e) => onSshKeyPathChange(e.currentTarget.value)}
                        />
                        <Button
                            variant="light"
                            leftSection={<IconFolder size={16} stroke={1.5} />}
                            onClick={handleBrowseKey}
                        >
                            {t("settings.vpn.ssh.browseKey")}
                        </Button>
                    </Group>
                )}

                <Switch
                    label={t("settings.vpn.ssh.autoConnect")}
                    description={t("settings.vpn.ssh.autoConnectDescription")}
                    checked={sshAutoConnect}
                    onChange={(e) => onSshAutoConnectChange(e.currentTarget.checked)}
                />

                {hostKeyMismatch && (
                    <Alert
                        color="red"
                        icon={<IconAlertTriangle size={16} />}
                        title={t("settings.vpn.ssh.hostKeyChanged")}
                    >
                        <Text size="sm" mb="xs">
                            {t("settings.vpn.ssh.hostKeyChangedMessage", {
                                host: sshHost,
                                port: sshPort || 22,
                            })}
                        </Text>
                        <Button
                            size="xs"
                            color="red"
                            variant="light"
                            onClick={handleResetHostKey}
                        >
                            {t("settings.vpn.ssh.resetHostKey")}
                        </Button>
                    </Alert>
                )}

                <Group gap="sm" align="center">
                    {reconnecting ? (
                        <>
                            <Badge color="yellow" variant="filled" size="sm">
                                {t("settings.vpn.ssh.reconnecting")} ({reconnectAttempt}/5)
                            </Badge>
                            <Button
                                variant="light"
                                color="red"
                                leftSection={<IconPlugConnectedX size={16} stroke={1.5} />}
                                onClick={handleDisconnect}
                                size="xs"
                            >
                                {t("settings.vpn.ssh.disconnect")}
                            </Button>
                        </>
                    ) : tunnelStatus.connected ? (
                        <>
                            <Badge color="green" variant="filled" size="sm">
                                {t("settings.vpn.ssh.connected")}
                            </Badge>
                            <Text size="xs" c="dimmed">
                                socks5://127.0.0.1:{tunnelStatus.localPort}
                            </Text>
                            <Button
                                variant="light"
                                color="red"
                                leftSection={<IconPlugConnectedX size={16} stroke={1.5} />}
                                onClick={handleDisconnect}
                                size="xs"
                            >
                                {t("settings.vpn.ssh.disconnect")}
                            </Button>
                        </>
                    ) : (
                        <>
                            <Badge color="brand" variant="light" size="sm">
                                {t("settings.vpn.ssh.disconnected")}
                            </Badge>
                            <Button
                                variant="filled"
                                leftSection={<IconPlugConnected size={16} stroke={1.5} />}
                                loading={connecting}
                                onClick={handleConnect}
                                size="xs"
                                disabled={!sshHost || !sshUsername}
                            >
                                {t("settings.vpn.ssh.connect")}
                            </Button>
                        </>
                    )}
                </Group>
            </Stack>
        </Stack>
    );
}

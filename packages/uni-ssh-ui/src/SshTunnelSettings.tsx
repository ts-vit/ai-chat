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
import { open } from "@tauri-apps/plugin-dialog";
import { useSettings } from "@uni-fw/ui";
import { useSshTunnel } from "./useSshTunnel";

interface SshTunnelSettingsProps {
  extraConnectParams?: Record<string, unknown>;
  proxyUrlOverride?: string | null;
}

export function SshTunnelSettings({ extraConnectParams, proxyUrlOverride }: SshTunnelSettingsProps = {}) {
  const { t } = useTranslation();

  // Settings (auto-save via useSettings)
  const hostSetting = useSettings("ssh.host");
  const portSetting = useSettings("ssh.port");
  const usernameSetting = useSettings("ssh.username");
  const authTypeSetting = useSettings("ssh.auth_type");
  const passwordSetting = useSettings("ssh.password");
  const keyPathSetting = useSettings("ssh.key_path");
  const autoConnectSetting = useSettings("ssh.auto_connect");

  // Tunnel operations
  const tunnel = useSshTunnel(t);

  // Local state for text inputs (save onBlur)
  const [hostLocal, setHostLocal] = useState("");
  const [portLocal, setPortLocal] = useState<number | undefined>(22);
  const [usernameLocal, setUsernameLocal] = useState("");
  const [passwordLocal, setPasswordLocal] = useState("");
  const [keyPathLocal, setKeyPathLocal] = useState("");

  // Sync from settings on load
  useEffect(() => {
    if (hostSetting.value !== undefined && hostSetting.value !== null) {
      setHostLocal(hostSetting.value);
    }
  }, [hostSetting.value]);

  useEffect(() => {
    if (portSetting.value !== undefined && portSetting.value !== null) {
      setPortLocal(Number(portSetting.value) || 22);
    }
  }, [portSetting.value]);

  useEffect(() => {
    if (usernameSetting.value !== undefined && usernameSetting.value !== null) {
      setUsernameLocal(usernameSetting.value);
    }
  }, [usernameSetting.value]);

  useEffect(() => {
    if (passwordSetting.value !== undefined && passwordSetting.value !== null) {
      setPasswordLocal(passwordSetting.value);
    }
  }, [passwordSetting.value]);

  useEffect(() => {
    if (keyPathSetting.value !== undefined && keyPathSetting.value !== null) {
      setKeyPathLocal(keyPathSetting.value);
    }
  }, [keyPathSetting.value]);

  // Derived values
  const authType = authTypeSetting.value ?? "password";
  const autoConnect = autoConnectSetting.value === "true";

  // Save helpers
  const saveField = async (
    setting: { set: (v: string) => Promise<void> },
    value: string
  ) => {
    await setting.set(value);
    notifications.show({
      message: t("settings.vpn.ssh.saved"),
      color: "green",
      autoClose: 2000,
    });
  };

  const handleHostBlur = () => {
    if (hostLocal !== (hostSetting.value ?? "")) {
      saveField(hostSetting, hostLocal);
    }
  };

  const handlePortBlur = () => {
    const current = portSetting.value !== null ? Number(portSetting.value) : 22;
    if (portLocal !== current) {
      saveField(portSetting, String(portLocal ?? 22));
    }
  };

  const handleUsernameBlur = () => {
    if (usernameLocal !== (usernameSetting.value ?? "")) {
      saveField(usernameSetting, usernameLocal);
    }
  };

  const handlePasswordBlur = () => {
    if (passwordLocal !== (passwordSetting.value ?? "")) {
      saveField(passwordSetting, passwordLocal);
    }
  };

  const handleKeyPathBlur = () => {
    if (keyPathLocal !== (keyPathSetting.value ?? "")) {
      saveField(keyPathSetting, keyPathLocal);
    }
  };

  const handleAuthTypeChange = (value: string) => {
    authTypeSetting.set(value);
  };

  const handleAutoConnectChange = (checked: boolean) => {
    autoConnectSetting.set(String(checked));
  };

  const handleConnect = async () => {
    const host = hostSetting.value ?? "";
    const port = portSetting.value ? Number(portSetting.value) : 22;
    const username = usernameSetting.value ?? "";
    const currentAuthType = authTypeSetting.value ?? "password";
    const password =
      currentAuthType === "key" ? null : passwordSetting.value || null;
    const privateKey =
      currentAuthType === "key" ? keyPathSetting.value || null : null;

    await tunnel.connect({
      host,
      port,
      username,
      authType: currentAuthType,
      password,
      privateKey,
      extraParams: extraConnectParams,
    });
  };

  const handleDisconnect = async () => {
    await tunnel.disconnect();
  };

  const handleResetHostKey = async () => {
    try {
      const host = hostSetting.value ?? "";
      const port = portSetting.value ? Number(portSetting.value) : 22;
      await tunnel.removeKnownHost(host, port);
      tunnel.setHostKeyMismatch(false);
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
      filters: [
        { name: "SSH Key", extensions: ["pem", "key", "ppk", "*"] },
      ],
    });
    if (result) {
      setKeyPathLocal(result as string);
      saveField(keyPathSetting, result as string);
    }
  };

  const canConnect = !!(hostSetting.value && usernameSetting.value);

  return (
    <Stack gap="lg">
      <Title order={4}>{t("settings.vpn.title")}</Title>

      <Stack gap="sm">
        <Group align="flex-end">
          <TextInput
            style={{ flex: 1 }}
            label={t("settings.vpn.ssh.host")}
            placeholder={t("settings.vpn.ssh.hostPlaceholder")}
            value={hostLocal}
            onChange={(e) => setHostLocal(e.currentTarget.value)}
            onBlur={handleHostBlur}
          />
          <NumberInput
            w={100}
            label={t("settings.vpn.ssh.port")}
            value={portLocal ?? 22}
            onChange={(value) =>
              setPortLocal(typeof value === "number" ? value : undefined)
            }
            onBlur={handlePortBlur}
            min={1}
            max={65535}
          />
        </Group>

        <TextInput
          label={t("settings.vpn.ssh.username")}
          placeholder={t("settings.vpn.ssh.usernamePlaceholder")}
          value={usernameLocal}
          onChange={(e) => setUsernameLocal(e.currentTarget.value)}
          onBlur={handleUsernameBlur}
        />

        <SegmentedControl
          value={authType}
          onChange={handleAuthTypeChange}
          data={[
            {
              value: "password",
              label: t("settings.vpn.ssh.authPassword"),
            },
            { value: "key", label: t("settings.vpn.ssh.authKey") },
          ]}
          size="xs"
        />

        {authType === "password" ? (
          <PasswordInput
            label={t("settings.vpn.ssh.password")}
            placeholder={t("settings.vpn.ssh.passwordPlaceholder")}
            value={passwordLocal}
            onChange={(e) => setPasswordLocal(e.currentTarget.value)}
            onBlur={handlePasswordBlur}
          />
        ) : (
          <Group align="flex-end">
            <TextInput
              style={{ flex: 1 }}
              label={t("settings.vpn.ssh.keyPath")}
              placeholder={t("settings.vpn.ssh.keyPathPlaceholder")}
              value={keyPathLocal}
              onChange={(e) => setKeyPathLocal(e.currentTarget.value)}
              onBlur={handleKeyPathBlur}
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
          checked={autoConnect}
          onChange={(e) => handleAutoConnectChange(e.currentTarget.checked)}
        />

        {tunnel.hostKeyMismatch && (
          <Alert
            color="red"
            icon={<IconAlertTriangle size={16} />}
            title={t("settings.vpn.ssh.hostKeyChanged")}
          >
            <Text size="sm" mb="xs">
              {t("settings.vpn.ssh.hostKeyChangedMessage", {
                host: hostSetting.value ?? "",
                port: portSetting.value
                  ? Number(portSetting.value)
                  : 22,
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
          {tunnel.reconnecting ? (
            <>
              <Badge color="yellow" variant="filled" size="sm">
                {t("settings.vpn.ssh.reconnecting")} (
                {tunnel.reconnectAttempt}/5)
              </Badge>
              <Button
                variant="light"
                color="red"
                leftSection={
                  <IconPlugConnectedX size={16} stroke={1.5} />
                }
                onClick={handleDisconnect}
                size="xs"
              >
                {t("settings.vpn.ssh.disconnect")}
              </Button>
            </>
          ) : tunnel.status.connected ? (
            <>
              <Badge color="green" variant="filled" size="sm">
                {t("settings.vpn.ssh.connected")}
              </Badge>
              <Text size="xs" c="dimmed">
                {proxyUrlOverride ?? `socks5://127.0.0.1:${tunnel.status.localPort}`}
              </Text>
              <Button
                variant="light"
                color="red"
                leftSection={
                  <IconPlugConnectedX size={16} stroke={1.5} />
                }
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
                leftSection={
                  <IconPlugConnected size={16} stroke={1.5} />
                }
                loading={tunnel.connecting}
                onClick={handleConnect}
                size="xs"
                disabled={!canConnect}
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

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { notifications } from "@mantine/notifications";
import type { SshTunnelStatus } from "./types";

interface SshTunnelState {
  status: SshTunnelStatus;
  connecting: boolean;
  reconnecting: boolean;
  reconnectAttempt: number;
  hostKeyMismatch: boolean;
  setHostKeyMismatch: (value: boolean) => void;
  connect: (config: {
    host: string;
    port: number;
    username: string;
    authType: string;
    password: string | null;
    privateKey: string | null;
    extraParams?: Record<string, unknown>;
  }) => Promise<number>;
  disconnect: () => Promise<void>;
  removeKnownHost: (host: string, port: number) => Promise<void>;
}

export function useSshTunnel(t: (key: string, params?: Record<string, unknown>) => string): SshTunnelState {
  const [status, setStatus] = useState<SshTunnelStatus>({
    connected: false,
    localPort: null,
    remoteHost: null,
  });
  const [connecting, setConnecting] = useState(false);
  const [reconnecting, setReconnecting] = useState(false);
  const [reconnectAttempt, setReconnectAttempt] = useState(0);
  const [hostKeyMismatch, setHostKeyMismatch] = useState(false);

  useEffect(() => {
    invoke<SshTunnelStatus>("ssh_tunnel_status")
      .then(setStatus)
      .catch(() => {});

    const unlistenConnected = listen<{ host: string; port: number }>(
      "ssh-tunnel-connected",
      (event) => {
        setStatus({
          connected: true,
          localPort: event.payload.port,
          remoteHost: event.payload.host,
        });
      }
    );

    const unlistenDisconnected = listen("ssh-tunnel-disconnected", () => {
      setStatus({ connected: false, localPort: null, remoteHost: null });
      setReconnecting(false);
      setReconnectAttempt(0);
    });

    const unlistenReconnecting = listen("ssh-tunnel-reconnecting", () => {
      setReconnecting(true);
      setReconnectAttempt(0);
    });

    const unlistenReconnectAttempt = listen<{
      attempt: number;
      maxAttempts: number;
    }>("ssh-tunnel-reconnect-attempt", (event) => {
      setReconnectAttempt(event.payload.attempt);
    });

    const unlistenReconnected = listen<{ port: number }>(
      "ssh-tunnel-reconnected",
      (event) => {
        setReconnecting(false);
        setReconnectAttempt(0);
        setStatus((prev) => ({
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

    const unlistenReconnectFailed = listen(
      "ssh-tunnel-reconnect-failed",
      () => {
        setReconnecting(false);
        setReconnectAttempt(0);
        notifications.show({
          message: t("settings.vpn.ssh.reconnectFailed"),
          color: "red",
          autoClose: 10000,
        });
      }
    );

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

  const connect = useCallback(
    async (config: {
      host: string;
      port: number;
      username: string;
      authType: string;
      password: string | null;
      privateKey: string | null;
      extraParams?: Record<string, unknown>;
    }) => {
      setConnecting(true);
      try {
        const { extraParams, ...baseConfig } = config;
        const port = await invoke<number>("ssh_tunnel_connect", { ...baseConfig, ...extraParams });
        notifications.show({
          message: t("settings.vpn.ssh.connectSuccess", { port }),
          color: "green",
          autoClose: 3000,
        });
        return port;
      } catch (e: unknown) {
        notifications.show({
          message: t("settings.vpn.ssh.connectError") + ": " + String(e),
          color: "red",
          autoClose: 5000,
        });
        throw e;
      } finally {
        setConnecting(false);
      }
    },
    [t]
  );

  const disconnect = useCallback(async () => {
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
  }, [t]);

  const removeKnownHost = useCallback(
    async (host: string, port: number) => {
      await invoke("ssh_remove_known_host", { host, port });
    },
    []
  );

  return {
    status,
    connecting,
    reconnecting,
    reconnectAttempt,
    hostKeyMismatch,
    setHostKeyMismatch,
    connect,
    disconnect,
    removeKnownHost,
  };
}

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Box,
    Group,
    SegmentedControl,
    Stack,
    Text,
    Tooltip,
} from "@mantine/core";
import {
    IconTrash,
    IconShieldCheck,
} from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import logo from "../assets/sp-logo.png";
import { MODE_DEFINITIONS } from "../constants/modes";
import { useChatStore } from "../store/chatStore";
import type { Chat, Comparison } from "../types";
import { ConfirmModal } from "./ConfirmModal";

interface AppHeaderProps {
    activeChat: Chat | undefined;
    activeComparison?: Comparison | null;
    effectiveLeftWidth: number;
    isNarrow: boolean;
    isVeryNarrow: boolean;
}

export function AppHeader({
    activeChat: _activeChat,
    activeComparison,
    effectiveLeftWidth,
    isNarrow,
}: AppHeaderProps) {
    const { t } = useTranslation();
    const {
        activeMode,
        setActiveMode,
        deleteComparison,
        loadMcpConnections,
    } = useChatStore();

    const [compareDeleteConfirmOpen, setCompareDeleteConfirmOpen] = useState(false);
    const [sshConnected, setSshConnected] = useState(false);
    const [sshRemoteHost, setSshRemoteHost] = useState("");

    useEffect(() => {
        loadMcpConnections();
    }, [loadMcpConnections]);

    useEffect(() => {
        invoke<{ connected: boolean; remoteHost: string | null }>("ssh_tunnel_status")
            .then((status) => {
                setSshConnected(status.connected);
                setSshRemoteHost(status.remoteHost ?? "");
            })
            .catch(() => {});

        const unConnected = listen<{ host: string; port: number }>(
            "ssh-tunnel-connected",
            (e) => {
                setSshConnected(true);
                setSshRemoteHost(e.payload.host);
            }
        );
        const unDisconnected = listen("ssh-tunnel-disconnected", () => {
            setSshConnected(false);
            setSshRemoteHost("");
        });

        return () => {
            unConnected.then((f) => f());
            unDisconnected.then((f) => f());
        };
    }, []);

    const leftZoneWidth = effectiveLeftWidth;

    return (
        <Box
            className="app-header"
            style={{
                height: 48,
                flexShrink: 0,
                borderBottom:
                    "1px solid var(--mantine-color-default-border)",
                background: "var(--mantine-color-body)",
                display: "flex",
                alignItems: "center",
                width: "100%",
            }}
        >
            {/* Left zone */}
            <Group
                gap="xs"
                style={{
                    width: leftZoneWidth,
                    minWidth: leftZoneWidth,
                    paddingLeft: "var(--mantine-spacing-xs)",
                    paddingRight: "var(--mantine-spacing-xs)",
                    flexShrink: 0,
                }}
                wrap="nowrap"
            >
                <img
                    src={logo}
                    alt=""
                    style={{ height: 40, display: "block" }}
                />
                {!isNarrow && (
                    <Text
                        fw={700}
                        style={{
                            fontFamily: "'JetBrains Mono', monospace",
                            fontSize: 16,
                            letterSpacing: 1.5,
                            whiteSpace: "nowrap",
                            color: "#D4854A",
                        }}
                    >
                        UNI AI
                    </Text>
                )}
            </Group>

            {/* Center zone — chat or compare */}
            {activeComparison !== undefined ? (
                <Group
                    gap="sm"
                    style={{
                        flex: 1,
                        justifyContent: "center",
                        alignItems: "center",
                        minWidth: 0,
                        paddingLeft: "var(--mantine-spacing-xs)",
                        paddingRight: "var(--mantine-spacing-xs)",
                    }}
                    wrap="nowrap"
                >
                    <Stack gap={0} style={{ flex: 1, minWidth: 0 }} align="center">
                        <Text size="sm" fw={600} lineClamp={1}>
                            {activeComparison?.title ?? t("compare.title")}
                        </Text>
                        {activeComparison && (
                            <Text size="xs" c="dimmed">
                                {activeComparison.leftModel.split("/").pop()} {t("compare.vsLabel")}{" "}
                                {activeComparison.rightModel.split("/").pop()}
                            </Text>
                        )}
                    </Stack>
                    {activeComparison && (
                        <Tooltip label={t("common.delete")}>
                            <ActionIcon
                                variant="subtle"
                                size="sm"
                                color="red"
                                onClick={() => setCompareDeleteConfirmOpen(true)}
                                aria-label={t("common.delete")}
                            >
                                <IconTrash size={16} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    )}
                </Group>
            ) : (
                <Group
                    gap="sm"
                    style={{
                        flex: 1,
                        justifyContent: "center",
                        alignItems: "center",
                        minWidth: 0,
                        paddingLeft: "var(--mantine-spacing-xs)",
                        paddingRight: "var(--mantine-spacing-xs)",
                    }}
                    wrap="nowrap"
                >
                    <SegmentedControl
                        value={activeMode}
                        onChange={(val) => setActiveMode(val)}
                        data={MODE_DEFINITIONS.map((m) => ({
                            value: m.id,
                            label: (
                                <Group gap={6} wrap="nowrap">
                                    <m.icon size={16} stroke={1.5} />
                                    <span>{t(m.labelKey)}</span>
                                </Group>
                            ),
                        }))}
                        size="sm"
                    />
                </Group>
            )}

            {/* Right zone */}
            <Group
                gap="xs"
                style={{
                    paddingRight: "var(--mantine-spacing-xs)",
                    flexShrink: 0,
                }}
            >
                {sshConnected && (
                    <Tooltip label={`VPN: ${sshRemoteHost}`}>
                        <IconShieldCheck size={16} stroke={1.5} color="var(--mantine-color-green-6)" />
                    </Tooltip>
                )}
            </Group>

            {activeComparison !== undefined && (
                <ConfirmModal
                    opened={compareDeleteConfirmOpen}
                    onClose={() => setCompareDeleteConfirmOpen(false)}
                    onConfirm={() => {
                        if (activeComparison) {
                            deleteComparison(activeComparison.id);
                        }
                        setCompareDeleteConfirmOpen(false);
                    }}
                    message={t("compare.deleteConfirm")}
                />
            )}

        </Box>
    );
}

import { useEffect, useRef, useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Box,
    Group,
    Menu,
    Stack,
    Text,
    Tooltip,
    UnstyledButton,
} from "@mantine/core";
import { useMantineColorScheme } from "@mantine/core";
import {
    IconTrash,
    IconShieldCheck,
    IconPlus,
    IconBriefcase,
    IconFolderPlus,
    IconArrowsExchange,
    IconSearch,
    IconList,
    IconBook2,
    IconBrain,
    IconWand,
    IconSubtask,
    IconCalendarEvent,
    IconDatabase,
    IconSun,
    IconMoon,
    IconTerminal2,
    IconSettings,
    IconChevronDown,
    IconLayoutSidebarLeftExpand,
    IconLayoutSidebarLeftCollapse,
    IconDots,
} from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { MODE_DEFINITIONS } from "../constants/modes";
import { useChatStore } from "../store/chatStore";
import type { Chat, Comparison } from "../types";
import { ConfirmModal } from "./ConfirmModal";
import { CreateFolderModal } from "./CreateFolderModal";
import { CreateProjectModal } from "./CreateProjectModal";

interface HeaderButton {
    id: string;
    icon: React.ComponentType<{ size?: number; stroke?: number }>;
    tooltipKey: string;
    onClick: () => void;
    active?: boolean;
}

interface AppHeaderProps {
    activeChat: Chat | undefined;
    activeComparison?: Comparison | null;
    onNewChat: () => void;
    onToggleSidebar: () => void;
    leftSidebarOpen: boolean;
    terminalOpen: boolean;
    onToggleTerminal: () => void;
}

export function AppHeader({
    activeChat: _activeChat,
    activeComparison,
    onNewChat,
    onToggleSidebar,
    leftSidebarOpen,
    terminalOpen,
    onToggleTerminal,
}: AppHeaderProps) {
    const { t } = useTranslation();
    const { colorScheme, toggleColorScheme } = useMantineColorScheme();
    const {
        activeMode,
        setActiveMode,
        deleteComparison,
        loadMcpConnections,
        currentView,
        setView,
    } = useChatStore();

    const [compareDeleteConfirmOpen, setCompareDeleteConfirmOpen] = useState(false);
    const [sshConnected, setSshConnected] = useState(false);
    const [sshRemoteHost, setSshRemoteHost] = useState("");
    const [createFolderModalOpen, setCreateFolderModalOpen] = useState(false);
    const [createProjectModalOpen, setCreateProjectModalOpen] = useState(false);

    const containerRef = useRef<HTMLDivElement>(null);
    const [visibleCount, setVisibleCount] = useState(Infinity);

    const isAssistantMode = activeMode === "assistant";

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

    // Overflow detection
    useEffect(() => {
        const container = containerRef.current;
        if (!container) return;

        const observer = new ResizeObserver(() => {
            const available = container.offsetWidth - 44;
            const count = Math.floor(available / 40);
            setVisibleCount(count);
        });

        observer.observe(container);
        return () => observer.disconnect();
    }, []);

    const headerButtons: HeaderButton[] = useMemo(() => [
        { id: "newChat", icon: IconPlus, tooltipKey: "sidebar.newChat", onClick: () => onNewChat() },
        {
            id: "newProjectOrFolder",
            icon: isAssistantMode ? IconBriefcase : IconFolderPlus,
            tooltipKey: isAssistantMode ? "project.new" : "sidebar.newFolder",
            onClick: () => isAssistantMode ? setCreateProjectModalOpen(true) : setCreateFolderModalOpen(true),
        },
        { id: "comparisons", icon: IconArrowsExchange, tooltipKey: "sidebar.comparisons", onClick: () => setView("comparisons"), active: currentView === "comparisons" },
        { id: "search", icon: IconSearch, tooltipKey: "common.search", onClick: () => setView("search"), active: currentView === "search" },
        { id: "snippets", icon: IconList, tooltipKey: "sidebar.snippets", onClick: () => setView("snippets"), active: currentView === "snippets" },
        { id: "promptLibrary", icon: IconBook2, tooltipKey: "sidebar.promptLibrary", onClick: () => setView("promptLibrary"), active: currentView === "promptLibrary" },
        { id: "memory", icon: IconBrain, tooltipKey: "sidebar.memory", onClick: () => setView("memory"), active: currentView === "memory" },
        { id: "skills", icon: IconWand, tooltipKey: "sidebar.skills", onClick: () => setView("skills"), active: currentView === "skills" },
        { id: "plans", icon: IconSubtask, tooltipKey: "sidebar.plans", onClick: () => setView("plans"), active: currentView === "plans" },
        { id: "scheduler", icon: IconCalendarEvent, tooltipKey: "scheduler.title", onClick: () => setView("scheduler"), active: currentView === "scheduler" },
        { id: "knowledgeBases", icon: IconDatabase, tooltipKey: "kb.title", onClick: () => setView("knowledgeBases"), active: currentView === "knowledgeBases" },
        { id: "theme", icon: colorScheme === "dark" ? IconSun : IconMoon, tooltipKey: "sidebar.toggleTheme", onClick: () => toggleColorScheme() },
        { id: "terminal", icon: IconTerminal2, tooltipKey: "terminal.tooltip", onClick: onToggleTerminal, active: terminalOpen },
        { id: "settings", icon: IconSettings, tooltipKey: "sidebar.settings", onClick: () => setView("settings"), active: currentView === "settings" },
    ], [isAssistantMode, currentView, colorScheme, terminalOpen, onNewChat, onToggleTerminal, setView, toggleColorScheme]);

    const visible = headerButtons.slice(0, visibleCount);
    const overflow = headerButtons.slice(visibleCount);

    const currentModeDefinition = MODE_DEFINITIONS.find((m) => m.id === activeMode) ?? MODE_DEFINITIONS[0];

    return (
        <Box
            className="app-header"
            style={{
                height: 48,
                flexShrink: 0,
                borderBottom: "1px solid var(--mantine-color-default-border)",
                background: "var(--mantine-color-body)",
                display: "flex",
                alignItems: "center",
                width: "100%",
            }}
        >
            {/* Left: sidebar toggle + mode dropdown + SSH indicator */}
            <Group
                gap="xs"
                style={{
                    paddingLeft: "var(--mantine-spacing-xs)",
                    paddingRight: "var(--mantine-spacing-xs)",
                    flexShrink: 0,
                }}
                wrap="nowrap"
            >
                <Tooltip label={leftSidebarOpen ? t("sidebar.collapse") : t("sidebar.expand")}>
                    <ActionIcon variant="subtle" size="lg" onClick={onToggleSidebar}>
                        {leftSidebarOpen
                            ? <IconLayoutSidebarLeftCollapse size={20} stroke={1.5} />
                            : <IconLayoutSidebarLeftExpand size={20} stroke={1.5} />
                        }
                    </ActionIcon>
                </Tooltip>

                <Menu position="bottom-start">
                    <Menu.Target>
                        <UnstyledButton
                            style={{
                                display: "flex",
                                alignItems: "center",
                                gap: 4,
                                padding: "4px 8px",
                                borderRadius: "var(--mantine-radius-sm)",
                            }}
                        >
                            <currentModeDefinition.icon size={16} stroke={1.5} />
                            <Text size="sm" fw={500}>{t(currentModeDefinition.labelKey)}</Text>
                            <IconChevronDown size={14} stroke={1.5} />
                        </UnstyledButton>
                    </Menu.Target>
                    <Menu.Dropdown>
                        {MODE_DEFINITIONS.map((m) => (
                            <Menu.Item
                                key={m.id}
                                leftSection={<m.icon size={16} stroke={1.5} />}
                                onClick={() => setActiveMode(m.id)}
                            >
                                {t(m.labelKey)}
                            </Menu.Item>
                        ))}
                    </Menu.Dropdown>
                </Menu>

                {sshConnected && (
                    <Tooltip label={`VPN: ${sshRemoteHost}`}>
                        <IconShieldCheck size={16} stroke={1.5} color="var(--mantine-color-green-6)" />
                    </Tooltip>
                )}
            </Group>

            {/* Center: toolbar buttons or comparison info */}
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
                    ref={containerRef}
                    gap={4}
                    wrap="nowrap"
                    justify="center"
                    style={{ flex: 1, overflow: "hidden", minWidth: 0 }}
                >
                    {visible.map((btn) => (
                        <Tooltip key={btn.id} label={t(btn.tooltipKey)}>
                            <ActionIcon
                                variant="subtle"
                                size="lg"
                                color={btn.active ? "brand" : undefined}
                                onClick={btn.onClick}
                            >
                                <btn.icon size={20} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    ))}
                    {overflow.length > 0 && (
                        <Menu position="bottom-end">
                            <Menu.Target>
                                <Tooltip label={t("header.more")}>
                                    <ActionIcon variant="subtle" size="lg">
                                        <IconDots size={20} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            </Menu.Target>
                            <Menu.Dropdown>
                                {overflow.map((btn) => (
                                    <Menu.Item
                                        key={btn.id}
                                        leftSection={<btn.icon size={18} stroke={1.5} />}
                                        onClick={btn.onClick}
                                        color={btn.active ? "brand" : undefined}
                                    >
                                        {t(btn.tooltipKey)}
                                    </Menu.Item>
                                ))}
                            </Menu.Dropdown>
                        </Menu>
                    )}
                </Group>
            )}

            {/* Right: empty placeholder for balance */}
            <Group
                gap="xs"
                style={{
                    paddingRight: "var(--mantine-spacing-xs)",
                    flexShrink: 0,
                    minWidth: 8,
                }}
            />

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

            <CreateFolderModal
                opened={createFolderModalOpen}
                onClose={() => setCreateFolderModalOpen(false)}
            />
            <CreateProjectModal
                opened={createProjectModalOpen}
                onClose={() => setCreateProjectModalOpen(false)}
            />
        </Box>
    );
}

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Button,
    Checkbox,
    Collapse,
    Group,
    NumberInput,
    Paper,
    Stack,
    Table,
    Text,
    Textarea,
    Tooltip,
} from "@mantine/core";
import { IconFolderPlus, IconTrash, IconChevronDown, IconChevronRight } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useChatStore } from "../../store/chatStore";
import type { FsMcpConfig } from "../../types";

export function FilesystemSection() {
    const { t } = useTranslation();
    const {
        mcpServers,
        mcpConnections,
        fsConfig,
        fsAuditLog,
        loadMcpServers,
        loadMcpConnections,
        loadFsConfig,
        saveFsConfig,
        loadFsAuditLog,
        clearFsAuditLog,
        toggleMcpServer,
    } = useChatStore();

    const [auditOpen, setAuditOpen] = useState(false);

    useEffect(() => {
        loadFsConfig();
        loadMcpServers();
        loadMcpConnections();
    }, [loadFsConfig, loadMcpServers, loadMcpConnections]);

    const server = mcpServers.find((s) => s.id === "builtin-filesystem");
    const connection = mcpConnections.find((c) => c.id === "builtin-filesystem");
    const isEnabled = server?.enabled ?? false;
    const isConnected = connection?.connected ?? false;
    const toolCount = connection?.toolCount ?? 0;

    const config: FsMcpConfig = fsConfig ?? {
        allowedDirectories: [],
        blockedPatterns: [".env", ".ssh", "*.key", "*.pem", "id_rsa", ".git/config"],
        readOnly: false,
        maxFileSizeBytes: 10485760,
        confirmDestructive: true,
    };

    const updateConfig = (partial: Partial<FsMcpConfig>) => {
        const updated = { ...config, ...partial };
        saveFsConfig(updated);
    };

    const handleToggle = async () => {
        await toggleMcpServer("builtin-filesystem", !isEnabled);
        await loadMcpConnections();
    };

    const handleAddDirectory = async () => {
        const selected = await open({ directory: true, multiple: false });
        if (selected) {
            const dir = typeof selected === "string" ? selected : selected;
            if (!config.allowedDirectories.includes(dir)) {
                updateConfig({
                    allowedDirectories: [...config.allowedDirectories, dir],
                });
            }
        }
    };

    const handleRemoveDirectory = (dir: string) => {
        updateConfig({
            allowedDirectories: config.allowedDirectories.filter((d) => d !== dir),
        });
    };

    const handleToggleAudit = () => {
        if (!auditOpen) {
            loadFsAuditLog(100);
        }
        setAuditOpen(!auditOpen);
    };

    const formatTimestamp = (ts: number) => {
        return new Date(ts * 1000).toLocaleString();
    };

    return (
        <Stack gap="lg">
            <Text size="sm" fw={500}>
                {t("filesystem.title")}
            </Text>
            <Text size="sm" c="dimmed">
                {t("filesystem.description")}
            </Text>

            <Paper p="md" withBorder>
                <Group justify="space-between">
                    <Group gap="sm">
                        <Text fw={500}>{t("filesystem.enabled")}</Text>
                        {isEnabled && (
                            <Badge
                                size="sm"
                                variant="light"
                                color={isConnected ? "green" : "red"}
                            >
                                {isConnected
                                    ? `${t("filesystem.connected")} — ${toolCount} ${t("filesystem.tools")}`
                                    : t("filesystem.disconnected")}
                            </Badge>
                        )}
                    </Group>
                    <Button
                        variant={isEnabled ? "filled" : "light"}
                        size="xs"
                        onClick={handleToggle}
                    >
                        {isEnabled ? t("filesystem.disconnect") : t("filesystem.connect")}
                    </Button>
                </Group>
            </Paper>

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("filesystem.allowedDirs")}
                </Text>
                {config.allowedDirectories.length === 0 ? (
                    <Text size="sm" c="dimmed">
                        {t("filesystem.noDirectories")}
                    </Text>
                ) : (
                    config.allowedDirectories.map((dir) => (
                        <Paper key={dir} p="xs" withBorder>
                            <Group justify="space-between" wrap="nowrap">
                                <Text size="sm" ff="monospace" lineClamp={1} style={{ minWidth: 0 }}>
                                    {dir}
                                </Text>
                                <Tooltip label={t("common.delete")}>
                                    <ActionIcon
                                        variant="subtle"
                                        color="red"
                                        size="xs"
                                        onClick={() => handleRemoveDirectory(dir)}
                                    >
                                        <IconTrash size={14} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            </Group>
                        </Paper>
                    ))
                )}
                <Button
                    variant="light"
                    size="xs"
                    leftSection={<IconFolderPlus size={16} stroke={1.5} />}
                    onClick={handleAddDirectory}
                    style={{ alignSelf: "flex-start" }}
                >
                    {t("filesystem.addDirectory")}
                </Button>
            </Stack>

            <Checkbox
                label={t("filesystem.readOnly")}
                description={t("filesystem.readOnlyHint")}
                checked={config.readOnly}
                onChange={(e) => updateConfig({ readOnly: e.currentTarget.checked })}
            />

            <NumberInput
                label={t("filesystem.maxFileSize")}
                value={Math.round(config.maxFileSizeBytes / 1048576)}
                onChange={(val) =>
                    updateConfig({ maxFileSizeBytes: (Number(val) || 10) * 1048576 })
                }
                min={1}
                max={100}
                suffix=" MB"
                style={{ maxWidth: 200 }}
            />

            <Checkbox
                label={t("filesystem.confirmDestructive")}
                description={t("filesystem.confirmDestructiveHint")}
                checked={config.confirmDestructive}
                onChange={(e) =>
                    updateConfig({ confirmDestructive: e.currentTarget.checked })
                }
            />

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("filesystem.blockedPatterns")}
                </Text>
                <Text size="xs" c="dimmed">
                    {t("filesystem.blockedPatternsHint")}
                </Text>
                <Textarea
                    value={config.blockedPatterns.join("\n")}
                    onChange={(e) =>
                        updateConfig({
                            blockedPatterns: e.currentTarget.value
                                .split("\n")
                                .map((s) => s.trim())
                                .filter(Boolean),
                        })
                    }
                    autosize
                    minRows={3}
                    maxRows={8}
                    placeholder=".env&#10;*.key&#10;.ssh"
                />
            </Stack>

            <Paper p="sm" withBorder>
                <Group
                    justify="space-between"
                    style={{ cursor: "pointer" }}
                    onClick={handleToggleAudit}
                >
                    <Text size="sm" fw={500}>
                        {t("filesystem.auditLog")}
                    </Text>
                    {auditOpen ? (
                        <IconChevronDown size={16} stroke={1.5} />
                    ) : (
                        <IconChevronRight size={16} stroke={1.5} />
                    )}
                </Group>
                <Collapse in={auditOpen}>
                    <Stack gap="sm" mt="sm">
                        {fsAuditLog.length === 0 ? (
                            <Text size="sm" c="dimmed">
                                {t("filesystem.noLogEntries")}
                            </Text>
                        ) : (
                            <Table striped highlightOnHover withTableBorder fz="xs">
                                <Table.Thead>
                                    <Table.Tr>
                                        <Table.Th>Time</Table.Th>
                                        <Table.Th>Tool</Table.Th>
                                        <Table.Th>Path</Table.Th>
                                        <Table.Th>Result</Table.Th>
                                    </Table.Tr>
                                </Table.Thead>
                                <Table.Tbody>
                                    {fsAuditLog.map((entry) => (
                                        <Table.Tr key={entry.id}>
                                            <Table.Td style={{ whiteSpace: "nowrap" }}>
                                                {formatTimestamp(entry.timestamp)}
                                            </Table.Td>
                                            <Table.Td>{entry.toolName}</Table.Td>
                                            <Table.Td>
                                                <Text size="xs" lineClamp={1} ff="monospace">
                                                    {entry.path}
                                                </Text>
                                            </Table.Td>
                                            <Table.Td>
                                                <Badge
                                                    size="xs"
                                                    color={entry.result === "ok" ? "green" : "red"}
                                                    variant="light"
                                                >
                                                    {entry.result}
                                                </Badge>
                                            </Table.Td>
                                        </Table.Tr>
                                    ))}
                                </Table.Tbody>
                            </Table>
                        )}
                        {fsAuditLog.length > 0 && (
                            <Button
                                variant="subtle"
                                color="red"
                                size="xs"
                                onClick={clearFsAuditLog}
                                style={{ alignSelf: "flex-start" }}
                            >
                                {t("filesystem.clearLog")}
                            </Button>
                        )}
                    </Stack>
                </Collapse>
            </Paper>
        </Stack>
    );
}

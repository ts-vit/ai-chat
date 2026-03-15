import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Button,
    Group,
    Modal,
    Paper,
    Stack,
    Switch,
    Text,
    Textarea,
    TextInput,
    Tooltip,
} from "@mantine/core";
import { IconPencil, IconPlus, IconRefresh, IconTrash } from "@tabler/icons-react";
import { useChatStore } from "../../store/chatStore";
import type { McpServer } from "../../types";
import { MCP_PRESETS, type McpPreset } from "../../constants/mcpPresets";
import { ConfirmModal } from "../ConfirmModal";
import { notify } from "../../utils/notify";

function getPackageName(args: string[]): string {
    return args.find((a) => a.startsWith("@")) ?? "";
}

export function McpSection() {
    const { t, i18n } = useTranslation();
    const {
        mcpServers,
        mcpConnections,
        loadMcpServers,
        loadMcpConnections,
        addMcpServer,
        updateMcpServer,
        removeMcpServer,
        toggleMcpServer,
        connectMcpServer,
    } = useChatStore();

    const [modalOpen, setModalOpen] = useState(false);
    const [editingServer, setEditingServer] = useState<McpServer | null>(null);
    const [deleteTarget, setDeleteTarget] = useState<McpServer | null>(null);
    const [reconnecting, setReconnecting] = useState(false);

    const [name, setName] = useState("");
    const [command, setCommand] = useState("");
    const [argsText, setArgsText] = useState("[]");
    const [envText, setEnvText] = useState("{}");
    const [nameError, setNameError] = useState<string | null>(null);
    const [commandError, setCommandError] = useState<string | null>(null);
    const [argsError, setArgsError] = useState<string | null>(null);
    const [envError, setEnvError] = useState<string | null>(null);

    useEffect(() => {
        loadMcpServers();
        loadMcpConnections();
    }, [loadMcpServers, loadMcpConnections]);

    const openForCreate = () => {
        setEditingServer(null);
        setName("");
        setCommand("");
        setArgsText("[]");
        setEnvText("{}");
        setNameError(null);
        setCommandError(null);
        setArgsError(null);
        setEnvError(null);
        setModalOpen(true);
    };

    const openForEdit = (server: McpServer) => {
        setEditingServer(server);
        setName(server.name);
        setCommand(server.command);
        setArgsText(JSON.stringify(server.args, null, 2));
        setEnvText(JSON.stringify(server.env, null, 2));
        setNameError(null);
        setCommandError(null);
        setArgsError(null);
        setEnvError(null);
        setModalOpen(true);
    };

    const closeModal = () => {
        setModalOpen(false);
        setEditingServer(null);
    };

    const openForPreset = (preset: McpPreset) => {
        setEditingServer(null);
        setName(preset.name);
        setCommand(preset.command);
        setArgsText(JSON.stringify(preset.args, null, 2));
        setEnvText(JSON.stringify(preset.env, null, 2));
        setNameError(null);
        setCommandError(null);
        setArgsError(null);
        setEnvError(null);
        setModalOpen(true);
    };

    const isPresetAdded = useMemo(() => {
        return (preset: McpPreset) =>
            mcpServers.some(
                (s) =>
                    s.command === preset.command &&
                    getPackageName(s.args) === getPackageName(preset.args)
            );
    }, [mcpServers]);

    const handleAddPreset = async (preset: McpPreset) => {
        if (preset.requiresEnv.length > 0) {
            openForPreset(preset);
            return;
        }
        await addMcpServer(preset.name, preset.command, preset.args, preset.env);
    };

    const handleSave = async () => {
        const trimmedName = name.trim();
        const trimmedCommand = command.trim();
        let hasError = false;
        if (!trimmedName) {
            setNameError(t("common.fieldRequired"));
            hasError = true;
        } else {
            setNameError(null);
        }
        if (!trimmedCommand) {
            setCommandError(t("common.fieldRequired"));
            hasError = true;
        } else {
            setCommandError(null);
        }
        if (hasError) return;

        let parsedArgs: string[];
        let parsedEnv: Record<string, string>;

        try {
            parsedArgs = JSON.parse(argsText);
            if (!Array.isArray(parsedArgs)) throw new Error();
            setArgsError(null);
        } catch {
            setArgsError(t("settings.mcp.invalidJson"));
            return;
        }

        try {
            parsedEnv = JSON.parse(envText);
            if (typeof parsedEnv !== "object" || parsedEnv === null || Array.isArray(parsedEnv)) throw new Error();
            setEnvError(null);
        } catch {
            setEnvError(t("settings.mcp.invalidJson"));
            return;
        }

        if (editingServer) {
            await updateMcpServer(editingServer.id, trimmedName, trimmedCommand, parsedArgs, parsedEnv, editingServer.enabled);
        } else {
            await addMcpServer(trimmedName, trimmedCommand, parsedArgs, parsedEnv);
        }
        closeModal();
    };

    const handleReconnectAll = async () => {
        setReconnecting(true);
        try {
            const enabledServers = mcpServers.filter((s) => s.enabled);
            for (const server of enabledServers) {
                try {
                    await connectMcpServer(server.id);
                } catch (e) {
                    notify.error(String(e));
                }
            }
            await loadMcpConnections();
        } finally {
            setReconnecting(false);
        }
    };

    const getConnection = (serverId: string) =>
        mcpConnections.find((c) => c.id === serverId);

    const getStatusColor = (server: McpServer) => {
        if (!server.enabled) return "var(--mantine-color-gray-5)";
        const conn = getConnection(server.id);
        if (conn?.connected) return "var(--mantine-color-green-6)";
        return "var(--mantine-color-red-6)";
    };

    return (
        <Stack gap="lg">
            <Text size="sm" fw={500}>
                {t("settings.mcp.title")}
            </Text>
            <Text size="sm" c="dimmed">
                {t("settings.mcp.description")}
            </Text>

            {mcpServers.filter((s) => s.id !== "builtin-filesystem").length === 0 ? (
                <Text c="dimmed" ta="center">
                    {t("settings.mcp.noServers")}
                </Text>
            ) : (
                <Stack gap="sm">
                    {mcpServers.filter((s) => s.id !== "builtin-filesystem").map((server) => {
                        const conn = getConnection(server.id);
                        return (
                            <Paper key={server.id} p="md" withBorder>
                                <Group justify="space-between" wrap="nowrap">
                                    <Group gap="sm" wrap="nowrap" style={{ minWidth: 0 }}>
                                        <div
                                            style={{
                                                width: 10,
                                                height: 10,
                                                borderRadius: "50%",
                                                backgroundColor: getStatusColor(server),
                                                flexShrink: 0,
                                            }}
                                        />
                                        <Text fw={500} lineClamp={1}>
                                            {server.name}
                                        </Text>
                                        {conn?.connected && (
                                            <Badge size="xs" variant="light">
                                                {conn.toolCount} {t("settings.mcp.tools")}
                                            </Badge>
                                        )}
                                    </Group>
                                    <Group gap="xs" wrap="nowrap">
                                        <Switch
                                            checked={server.enabled}
                                            onChange={() => toggleMcpServer(server.id, !server.enabled)}
                                            size="sm"
                                        />
                                        <Tooltip label={t("common.edit")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                onClick={() => openForEdit(server)}
                                                aria-label={t("common.edit")}
                                            >
                                                <IconPencil size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("common.delete")}>
                                            <ActionIcon
                                                variant="subtle"
                                                size="xs"
                                                color="red"
                                                onClick={() => setDeleteTarget(server)}
                                                aria-label={t("common.delete")}
                                            >
                                                <IconTrash size={14} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                </Group>
                                <Text size="xs" c="dimmed" ff="monospace" mt={4}>
                                    {server.command} {server.args.join(" ")}
                                </Text>
                            </Paper>
                        );
                    })}
                </Stack>
            )}

            <Group gap="sm">
                <Button
                    variant="light"
                    leftSection={<IconPlus size={18} stroke={1.5} />}
                    onClick={openForCreate}
                >
                    {t("settings.mcp.addServer")}
                </Button>
                {mcpServers.some((s) => s.enabled) && (
                    <Button
                        variant="subtle"
                        leftSection={<IconRefresh size={18} stroke={1.5} />}
                        onClick={handleReconnectAll}
                        loading={reconnecting}
                    >
                        {t("settings.mcp.reconnectAll")}
                    </Button>
                )}
            </Group>

            <Stack gap="sm">
                <Text size="sm" fw={500}>
                    {t("settings.mcp.catalog")}
                </Text>
                <Text size="xs" c="dimmed">
                    {t("settings.mcp.catalogDescription")}
                </Text>
                <Stack gap="xs">
                    {MCP_PRESETS.map((preset) => {
                        const added = isPresetAdded(preset);
                        const description =
                            i18n.language?.startsWith("ru") ? preset.description : preset.descriptionEn;
                        return (
                            <Paper key={preset.name} p="md" withBorder>
                                <Group justify="space-between" wrap="nowrap">
                                    <Stack gap={2} style={{ minWidth: 0 }}>
                                        <Text fw={500} lineClamp={1}>
                                            {preset.name}
                                        </Text>
                                        <Text size="xs" c="dimmed" lineClamp={2}>
                                            {description}
                                        </Text>
                                    </Stack>
                                    {added ? (
                                        <Badge size="sm" variant="light" color="green">
                                            {t("settings.mcp.alreadyAdded")}
                                        </Badge>
                                    ) : (
                                        <Button
                                            variant="light"
                                            size="xs"
                                            onClick={() => handleAddPreset(preset)}
                                        >
                                            {t("settings.mcp.addPreset")}
                                        </Button>
                                    )}
                                </Group>
                            </Paper>
                        );
                    })}
                </Stack>
            </Stack>

            <Modal
                title={editingServer ? t("settings.mcp.editServer") : t("settings.mcp.addServer")}
                opened={modalOpen}
                onClose={closeModal}
                size="md"
            >
                <Stack gap="sm">
                    <TextInput
                        label={t("settings.mcp.serverName")}
                        placeholder="Filesystem Server"
                        value={name}
                        onChange={(e) => {
                            setName(e.currentTarget.value);
                            setNameError(null);
                        }}
                        error={nameError}
                        required
                    />
                    <TextInput
                        label={t("settings.mcp.serverCommand")}
                        placeholder="npx"
                        value={command}
                        onChange={(e) => {
                            setCommand(e.currentTarget.value);
                            setCommandError(null);
                        }}
                        error={commandError}
                        required
                    />
                    <Textarea
                        label={t("settings.mcp.serverArgs")}
                        placeholder='["--directory", "/path/to/dir"]'
                        value={argsText}
                        onChange={(e) => {
                            setArgsText(e.currentTarget.value);
                            setArgsError(null);
                        }}
                        error={argsError}
                        autosize
                        minRows={2}
                    />
                    <Textarea
                        label={t("settings.mcp.serverEnv")}
                        placeholder='{"API_KEY": "your-key"}'
                        value={envText}
                        onChange={(e) => {
                            setEnvText(e.currentTarget.value);
                            setEnvError(null);
                        }}
                        error={envError}
                        autosize
                        minRows={2}
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={closeModal}>
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={handleSave}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            <ConfirmModal
                opened={!!deleteTarget}
                onClose={() => setDeleteTarget(null)}
                onConfirm={async () => {
                    if (deleteTarget) {
                        await removeMcpServer(deleteTarget.id);
                        setDeleteTarget(null);
                    }
                }}
                message={
                    deleteTarget
                        ? t("settings.mcp.deleteConfirm", { name: deleteTarget.name })
                        : ""
                }
            />
        </Stack>
    );
}

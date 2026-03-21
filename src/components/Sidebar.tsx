import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Collapse,
    Group,
    Menu,
    Modal,
    NavLink,
    Popover,
    ScrollArea,
    Stack,
    Text,
    Textarea,
    TextInput,
    Loader,
    Tooltip,
    useMantineColorScheme,
} from "@mantine/core";
import { ColorSwatch } from "@mantine/core";
import {
    IconArchive,
    IconBriefcase,
    IconCheck,
    IconChevronDown,
    IconChevronRight,
    IconFolderFilled,
    IconMessages,
    IconPencil,
    IconPlayerStop,
    IconPlus,
    IconTrash,
    IconTarget,
    IconLayoutBoard,
    IconX,
} from "@tabler/icons-react";
import { FOLDER_COLORS } from "../constants/folderColors";
import { useChatStore } from "../store/chatStore";
import { formatRelativeDate } from "../utils/formatDate";
import { ConfirmModal } from "./ConfirmModal";
import { CreateFolderModal } from "./CreateFolderModal";
import { CreateProjectModal } from "./CreateProjectModal";
import type { Chat, Folder, ProjectSummary } from "../types";

interface SidebarProps {
    width?: number;
    style?: React.CSSProperties;
    compact?: boolean;
    onNewChat: (projectId?: string) => void;
}

function ChatStatusIcon({ status }: { status?: string | null }) {
    if (!status) return null;
    switch (status) {
        case "completed":
            return <IconCheck size={12} color="var(--mantine-color-green-5)" />;
        case "running":
            return <Loader size={12} color="var(--mantine-color-teal-5)" />;
        case "failed":
            return <IconX size={12} color="var(--mantine-color-red-5)" />;
        case "cancelled":
            return <IconPlayerStop size={12} color="var(--mantine-color-yellow-5)" />;
        case "paused_budget":
            return <IconPlayerStop size={12} color="var(--mantine-color-orange-5)" />;
        default:
            return null;
    }
}

function ChatRow({
    chat,
    activeChatId,
    onSelect,
    onDelete,
    onContextMenu,
    onMouseDown,
    isDragging,
    isActiveDrag,
    isContextMenuOpen,
}: {
    chat: Chat;
    activeChatId: string | null;
    onSelect: () => void;
    onDelete: () => void;
    onContextMenu?: (e: React.MouseEvent, chatId: string) => void;
    onMouseDown?: (e: React.MouseEvent) => void;
    isDragging?: boolean;
    isActiveDrag?: boolean;
    isContextMenuOpen?: boolean;
}) {
    const { t } = useTranslation();
    return (
        <div
            className="chat-item"
            role="listitem"
            data-context-menu-open={isContextMenuOpen ? "true" : undefined}
            onContextMenu={onContextMenu ? (e) => onContextMenu(e, chat.id) : undefined}
            onMouseDown={onMouseDown}
            style={{
                width: "100%",
                cursor: onMouseDown ? (isActiveDrag ? "grabbing" : "grab") : undefined,
                opacity: isDragging ? 0.5 : undefined,
                userSelect: "none",
            }}
        >
            <Group
                wrap="nowrap"
                gap={0}
                justify="flex-start"
            >
                <NavLink
                active={chat.id === activeChatId}
                label={chat.title}
                rightSection={chat.mode === "assistant" ? <ChatStatusIcon status={chat.lastRunStatus} /> : undefined}
                description={
                    chat.updatedAt != null ? (
                        <Text component="span" size="xs" c="dimmed">
                            {formatRelativeDate(chat.updatedAt)}
                        </Text>
                    ) : undefined
                }
                onClick={(e) => {
                    if (isDragging) {
                        e.preventDefault();
                        e.stopPropagation();
                        return;
                    }
                    onSelect();
                }}
                style={{ flex: 1, minWidth: 0 }}
                styles={{
                    root: {
                        borderLeft:
                            chat.id === activeChatId
                                ? "3px solid var(--mantine-color-brand-5)"
                                : "3px solid transparent",
                    },
                }}
            />
            <Tooltip label={t("sidebar.deleteChat")}>
                <ActionIcon
                    className="chat-delete-btn"
                    size="xs"
                    variant="subtle"
                    onClick={(e) => {
                        e.stopPropagation();
                        onDelete();
                    }}
                >
                    ✕
                </ActionIcon>
            </Tooltip>
            </Group>
        </div>
    );
}

export function Sidebar({ width = 260, style, compact = false, onNewChat }: SidebarProps) {
    const { t } = useTranslation();
    const { colorScheme } = useMantineColorScheme();

    const folderDropHighlightBg = useCallback(
        (folderColor: string | null) => {
            if (folderColor) {
                return colorScheme === "dark"
                    ? `var(--mantine-color-${folderColor}-9)`
                    : `var(--mantine-color-${folderColor}-1)`;
            }
            return colorScheme === "dark"
                ? "var(--mantine-color-dark-6)"
                : "var(--mantine-color-gray-1)";
        },
        [colorScheme]
    );
    const {
        chats,
        folders,
        projects,
        activeChatId,
        deleteChat,
        setActiveChat,
        setView,
        updateFolder,
        deleteFolder,
        moveChatToFolder,
        updateProject,
        deleteProject: storeDeleteProject,
        archiveProject,
        assignChatToProject,
        removeChatFromProject,
        openProjectDashboard,
    } = useChatStore();
    const activeMode = useChatStore((s) => s.activeMode);
    const agentMemories = useChatStore((s) => s.agentMemories);
    const knowledgeBases = useChatStore((s) => s.knowledgeBases);
    const skills = useChatStore((s) => s.skills);

    const isAssistantMode = activeMode === "assistant";
    const modeChats = chats.filter((c) => (c.mode ?? "chat") === activeMode);
    const modeFolders = folders.filter((f) => (f.mode ?? "chat") === activeMode);
    const activeProjects = projects.filter((p) => p.status !== "archived");

    const [deletingChatId, setDeletingChatId] = useState<string | null>(null);
    const [deletingFolderId, setDeletingFolderId] = useState<string | null>(null);
    const [chatsPopoverOpened, setChatsPopoverOpened] = useState(false);
    const [contextMenu, setContextMenu] = useState<{
        chatId: string;
        x: number;
        y: number;
    } | null>(null);
    const [expandedFolderIds, setExpandedFolderIds] = useState<Set<string>>(new Set());
    const [createFolderModalOpen, setCreateFolderModalOpen] = useState(false);
    const [editingFolderId, setEditingFolderId] = useState<string | null>(null);
    const [editingFolderName, setEditingFolderName] = useState("");
    const [editingFolderColor, setEditingFolderColor] = useState<string | null>(null);
    const [dragState, setDragState] = useState<{
        chatId: string;
        chatTitle: string;
        startX: number;
        startY: number;
        currentX: number;
        currentY: number;
        isDragging: boolean;
    } | null>(null);
    const [dragOverFolderId, setDragOverFolderId] = useState<string | null>(null);
    const [dragOverNoFolder, setDragOverNoFolder] = useState(false);

    // Project state (assistant mode)
    const [expandedProjectIds, setExpandedProjectIds] = useState<Set<string>>(new Set());
    const [createProjectModalOpen, setCreateProjectModalOpen] = useState(false);
    const [editingProjectId, setEditingProjectId] = useState<string | null>(null);
    const [editingProjectName, setEditingProjectName] = useState("");
    const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);
    const [editGoalProjectId, setEditGoalProjectId] = useState<string | null>(null);
    const [editGoalText, setEditGoalText] = useState("");
    const [dragOverProjectId, setDragOverProjectId] = useState<string | null>(null);
    const [dragOverNoProject, setDragOverNoProject] = useState(false);
    const [projectContextMenu, setProjectContextMenu] = useState<{
        projectId: string;
        x: number;
        y: number;
    } | null>(null);

    const toggleFolder = useCallback((folderId: string) => {
        setExpandedFolderIds((prev) => {
            const next = new Set(prev);
            if (next.has(folderId)) next.delete(folderId);
            else next.add(folderId);
            return next;
        });
    }, []);

    const startEditFolder = useCallback((folder: Folder) => {
        setEditingFolderId(folder.id);
        setEditingFolderName(folder.name);
        setEditingFolderColor(folder.color);
    }, []);

    const saveEditFolder = useCallback(() => {
        if (editingFolderId && editingFolderName.trim()) {
            updateFolder(editingFolderId, editingFolderName.trim(), editingFolderColor);
            setEditingFolderId(null);
            setEditingFolderName("");
            setEditingFolderColor(null);
        }
    }, [editingFolderId, editingFolderName, editingFolderColor, updateFolder]);

    const cancelEditFolder = useCallback(() => {
        setEditingFolderId(null);
        setEditingFolderName("");
        setEditingFolderColor(null);
    }, []);

    const handleChatContextMenu = useCallback((e: React.MouseEvent, chatId: string) => {
        e.preventDefault();
        e.stopPropagation();
        setContextMenu({ chatId, x: e.clientX, y: e.clientY });
    }, []);

    const chatsInFolder = useCallback(
        (folderId: string) => modeChats.filter((c) => c.folderId === folderId),
        [modeChats]
    );
    const chatsWithoutFolder = modeChats.filter((c) => !c.folderId);

    const chatsInProject = useCallback(
        (projectId: string) => modeChats.filter((c) => c.projectId === projectId),
        [modeChats]
    );
    const chatsWithoutProject = modeChats.filter((c) => !c.projectId);

    const toggleProject = useCallback((projectId: string) => {
        setExpandedProjectIds((prev) => {
            const next = new Set(prev);
            if (next.has(projectId)) next.delete(projectId);
            else next.add(projectId);
            return next;
        });
    }, []);

    const startEditProject = useCallback((project: ProjectSummary) => {
        setEditingProjectId(project.id);
        setEditingProjectName(project.name);
    }, []);

    const saveEditProject = useCallback(() => {
        if (editingProjectId && editingProjectName.trim()) {
            updateProject(editingProjectId, { name: editingProjectName.trim() });
            setEditingProjectId(null);
            setEditingProjectName("");
        }
    }, [editingProjectId, editingProjectName, updateProject]);

    const cancelEditProject = useCallback(() => {
        setEditingProjectId(null);
        setEditingProjectName("");
    }, []);

    const handleProjectContextMenu = useCallback((e: React.MouseEvent, projectId: string) => {
        e.preventDefault();
        e.stopPropagation();
        setProjectContextMenu({ projectId, x: e.clientX, y: e.clientY });
    }, []);

    const folderColorVar = (color: string | null) =>
        color ? `var(--mantine-color-${color}-5)` : "var(--mantine-color-gray-5)";

    const contextChat = contextMenu ? modeChats.find((c) => c.id === contextMenu.chatId) : null;

    const renderChatList = (
        chatList: Chat[],
        onSelect: () => void,
        enableDrag?: boolean
    ) =>
        chatList.map((chat) => (
            <ChatRow
                key={chat.id}
                chat={chat}
                activeChatId={activeChatId}
                onSelect={() => {
                    setActiveChat(chat.id);
                    setView("chat");
                    onSelect();
                }}
                onDelete={() => setDeletingChatId(chat.id)}
                onContextMenu={handleChatContextMenu}
                onMouseDown={
                    enableDrag
                        ? (e) => {
                              if (e.button !== 0) return;
                              if ((e.target as HTMLElement).closest(".chat-delete-btn")) return;
                              setDragState({
                                  chatId: chat.id,
                                  chatTitle: chat.title,
                                  startX: e.clientX,
                                  startY: e.clientY,
                                  currentX: e.clientX,
                                  currentY: e.clientY,
                                  isDragging: false,
                              });
                          }
                        : undefined
                }
                isDragging={dragState?.isDragging === true && dragState.chatId === chat.id}
                isActiveDrag={dragState !== null && dragState.chatId === chat.id}
                isContextMenuOpen={contextMenu?.chatId === chat.id}
            />
        ));

    const renderFolderList = () => (
        <>
            {modeFolders.map((folder) => {
                const count = chatsInFolder(folder.id).length;
                const isExpanded = expandedFolderIds.has(folder.id);
                const isEditing = editingFolderId === folder.id;

                const isDragOver = dragOverFolderId === folder.id;

                return (
                    <Box key={folder.id} mb="xs">
                        <div
                            className="folder-row"
                            data-folder-id={folder.id}
                            style={{
                                width: "100%",
                                borderRadius: 4,
                                padding: "2px 4px",
                                ...(isDragOver && {
                                    background: folderDropHighlightBg(folder.color),
                                }),
                            }}
                        >
                            <Group
                                wrap="nowrap"
                                gap="xs"
                                style={{ cursor: isEditing ? "default" : "pointer" }}
                                onClick={() => !isEditing && toggleFolder(folder.id)}
                            >
                            <Box style={{ width: 16 }}>
                                {isExpanded ? (
                                    <IconChevronDown size={14} stroke={1.5} />
                                ) : (
                                    <IconChevronRight size={14} stroke={1.5} />
                                )}
                            </Box>
                            <IconFolderFilled
                                size={18}
                                style={{
                                    color: folder.color
                                        ? folderColorVar(folder.color)
                                        : undefined,
                                }}
                            />
                            {isEditing ? (
                                <Box
                                    style={{ flex: 1, minWidth: 0 }}
                                    onClick={(e) => e.stopPropagation()}
                                >
                                    <TextInput
                                        size="xs"
                                        value={editingFolderName}
                                        onChange={(e) =>
                                            setEditingFolderName(e.currentTarget.value)
                                        }
                                        onKeyDown={(e) => {
                                            if (e.key === "Enter") saveEditFolder();
                                            if (e.key === "Escape") cancelEditFolder();
                                        }}
                                        autoFocus
                                        styles={{
                                            input: {
                                                color: editingFolderColor
                                                    ? folderColorVar(editingFolderColor)
                                                    : undefined,
                                            },
                                        }}
                                    />
                                    <Group gap={4} mt={4}>
                                        {FOLDER_COLORS.map((c) => (
                                            <ColorSwatch
                                                key={c}
                                                color={`var(--mantine-color-${c}-5)`}
                                                size={16}
                                                onClick={() =>
                                                    setEditingFolderColor(c)
                                                }
                                                style={{
                                                    cursor: "pointer",
                                                    border:
                                                        editingFolderColor === c
                                                            ? "2px solid var(--mantine-color-default-border)"
                                                            : undefined,
                                                }}
                                            />
                                        ))}
                                    </Group>
                                </Box>
                            ) : (
                                <>
                                    <Tooltip
                                        label={folder.name}
                                        disabled={folder.name.length < 15}
                                    >
                                        <Text
                                            size="sm"
                                            style={{
                                                flex: 1,
                                                minWidth: 0,
                                                color: folder.color
                                                    ? folderColorVar(folder.color)
                                                    : undefined,
                                            }}
                                            truncate
                                            onDoubleClick={(e) => {
                                                e.stopPropagation();
                                                startEditFolder(folder);
                                            }}
                                        >
                                            {folder.name}
                                        </Text>
                                    </Tooltip>
                                    <Text size="xs" c="dimmed">
                                        {count}
                                    </Text>
                                    <Group className="action-icons" gap={4} wrap="nowrap">
                                        <Tooltip label={t("sidebar.renameFolder")}>
                                            <ActionIcon
                                                size="xs"
                                                variant="subtle"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    startEditFolder(folder);
                                                }}
                                            >
                                                <IconPencil size={12} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                        <Tooltip label={t("sidebar.deleteFolder")}>
                                            <ActionIcon
                                                size="xs"
                                                variant="subtle"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    setDeletingFolderId(folder.id);
                                                }}
                                            >
                                                <IconTrash size={12} stroke={1.5} />
                                            </ActionIcon>
                                        </Tooltip>
                                    </Group>
                                </>
                            )}
                            </Group>
                        </div>
                        <Collapse in={isExpanded}>
                            <Box pl="md" pr="xs">
                                {renderChatList(
                                    chatsInFolder(folder.id),
                                    () => {},
                                    true
                                )}
                            </Box>
                        </Collapse>
                    </Box>
                );
            })}
        </>
    );

    const renderProjectList = () => (
        <>
            {activeProjects.map((project) => {
                const count = chatsInProject(project.id).length;
                const isExpanded = expandedProjectIds.has(project.id);
                const isEditing = editingProjectId === project.id;
                const isDragOver = dragOverProjectId === project.id;
                const isCompleted = project.status === "completed";

                return (
                    <Box key={project.id} mb="xs">
                        <div
                            className="folder-row"
                            data-project-id={project.id}
                            onContextMenu={(e) => handleProjectContextMenu(e, project.id)}
                            style={{
                                width: "100%",
                                borderRadius: 4,
                                padding: "2px 4px",
                                ...(isDragOver && {
                                    background: colorScheme === "dark"
                                        ? "var(--mantine-color-brand-9)"
                                        : "var(--mantine-color-brand-1)",
                                }),
                            }}
                        >
                            <Group
                                wrap="nowrap"
                                gap="xs"
                                style={{ cursor: isEditing ? "default" : "pointer" }}
                                onClick={() => !isEditing && openProjectDashboard(project.id)}
                            >
                                <Box
                                    style={{ width: 16, cursor: "pointer" }}
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        toggleProject(project.id);
                                    }}
                                >
                                    {isExpanded ? (
                                        <IconChevronDown size={14} stroke={1.5} />
                                    ) : (
                                        <IconChevronRight size={14} stroke={1.5} />
                                    )}
                                </Box>
                                <IconBriefcase
                                    size={18}
                                    stroke={1.5}
                                    style={{
                                        color: isCompleted
                                            ? "var(--mantine-color-green-5)"
                                            : "var(--mantine-color-brand-5)",
                                    }}
                                />
                                {isEditing ? (
                                    <Box
                                        style={{ flex: 1, minWidth: 0 }}
                                        onClick={(e) => e.stopPropagation()}
                                    >
                                        <TextInput
                                            size="xs"
                                            value={editingProjectName}
                                            onChange={(e) => setEditingProjectName(e.currentTarget.value)}
                                            onKeyDown={(e) => {
                                                if (e.key === "Enter") saveEditProject();
                                                if (e.key === "Escape") cancelEditProject();
                                            }}
                                            autoFocus
                                        />
                                    </Box>
                                ) : (
                                    <>
                                        <Tooltip
                                            label={project.goal || project.name}
                                            disabled={project.name.length < 15 && !project.goal}
                                        >
                                            <Text
                                                size="sm"
                                                style={{
                                                    flex: 1,
                                                    minWidth: 0,
                                                    ...(isCompleted && {
                                                        textDecoration: "line-through",
                                                        opacity: 0.7,
                                                    }),
                                                }}
                                                truncate
                                                onDoubleClick={(e) => {
                                                    e.stopPropagation();
                                                    startEditProject(project);
                                                }}
                                            >
                                                {project.name}
                                            </Text>
                                        </Tooltip>
                                        <Text size="xs" c="dimmed">
                                            {count}
                                        </Text>
                                        <Group className="action-icons" gap={4} wrap="nowrap">
                                            <Tooltip label={t("project.newChatInProject")}>
                                                <ActionIcon
                                                    size="xs"
                                                    variant="subtle"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        onNewChat(project.id);
                                                    }}
                                                >
                                                    <IconPlus size={12} stroke={1.5} />
                                                </ActionIcon>
                                            </Tooltip>
                                            <Tooltip label={t("project.rename")}>
                                                <ActionIcon
                                                    size="xs"
                                                    variant="subtle"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        startEditProject(project);
                                                    }}
                                                >
                                                    <IconPencil size={12} stroke={1.5} />
                                                </ActionIcon>
                                            </Tooltip>
                                            <Tooltip label={t("project.delete")}>
                                                <ActionIcon
                                                    size="xs"
                                                    variant="subtle"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        setDeletingProjectId(project.id);
                                                    }}
                                                >
                                                    <IconTrash size={12} stroke={1.5} />
                                                </ActionIcon>
                                            </Tooltip>
                                        </Group>
                                    </>
                                )}
                            </Group>
                        </div>
                        {isExpanded && project.goal && (
                            <Text size="xs" c="dimmed" pl={34} lineClamp={1} mb={2}>
                                {project.goal}
                            </Text>
                        )}
                        <Collapse in={isExpanded}>
                            <Box pl="md" pr="xs">
                                {renderChatList(
                                    chatsInProject(project.id),
                                    () => {},
                                    true
                                )}
                            </Box>
                        </Collapse>
                    </Box>
                );
            })}
        </>
    );

    const noFolderDropBg =
        colorScheme === "dark"
            ? "var(--mantine-color-dark-6)"
            : "var(--mantine-color-gray-1)";

    const noProjectDropBg =
        colorScheme === "dark"
            ? "var(--mantine-color-dark-6)"
            : "var(--mantine-color-gray-1)";

    const freeChats = isAssistantMode ? chatsWithoutProject : chatsWithoutFolder;
    const hasFreeChats = freeChats.length > 0;

    const mainContent = (
        <div style={{ flex: 1, overflowY: "auto" }}>
            {!compact && (isAssistantMode ? renderProjectList() : renderFolderList())}
            {!compact && <Box mb="xs" />}
            {!compact && isAssistantMode && hasFreeChats && (
                <Text size="xs" c="dimmed" px={4} mb={4}>
                    {t("project.freeChats")}
                </Text>
            )}
            {!compact ? (
                <Box
                    {...(isAssistantMode
                        ? { "data-no-project-zone": true }
                        : { "data-no-folder-zone": true })}
                    style={{
                        borderRadius: 4,
                        padding: "2px 4px",
                        minHeight: 8,
                        ...((isAssistantMode ? dragOverNoProject : dragOverNoFolder) && {
                            background: isAssistantMode ? noProjectDropBg : noFolderDropBg,
                        }),
                    }}
                >
                    {renderChatList(freeChats, () => {}, true)}
                </Box>
            ) : (
                renderChatList(freeChats, () => {})
            )}
        </div>
    );

    const popoverContent = (
        <ScrollArea.Autosize mah="calc(100vh - 200px)" type="scroll">
            {modeFolders.map((folder) => {
                const count = chatsInFolder(folder.id).length;
                const isExpanded = expandedFolderIds.has(folder.id);
                return (
                    <Box key={folder.id} mb="xs">
                        <Group
                            wrap="nowrap"
                            gap="xs"
                            style={{
                                cursor: "pointer",
                                padding: "2px 4px",
                                borderRadius: 4,
                            }}
                            onClick={() => toggleFolder(folder.id)}
                        >
                            <Box style={{ width: 16 }}>
                                {isExpanded ? (
                                    <IconChevronDown size={14} stroke={1.5} />
                                ) : (
                                    <IconChevronRight size={14} stroke={1.5} />
                                )}
                            </Box>
                            <IconFolderFilled
                                size={18}
                                style={{
                                    color: folder.color
                                        ? folderColorVar(folder.color)
                                        : undefined,
                                }}
                            />
                            <Tooltip
                                label={folder.name}
                                disabled={folder.name.length < 15}
                            >
                                <Text
                                    size="sm"
                                    style={{
                                        flex: 1,
                                        color: folder.color
                                            ? folderColorVar(folder.color)
                                            : undefined,
                                    }}
                                    truncate
                                >
                                    {folder.name}
                                </Text>
                            </Tooltip>
                            <Text size="xs" c="dimmed">
                                {count}
                            </Text>
                        </Group>
                        {isExpanded && (
                            <Box pl="md" pr="xs" mb="xs">
                                {renderChatList(
                                    chatsInFolder(folder.id),
                                    () => setChatsPopoverOpened(false)
                                )}
                            </Box>
                        )}
                    </Box>
                );
            })}
            {renderChatList(chatsWithoutFolder, () => setChatsPopoverOpened(false))}
        </ScrollArea.Autosize>
    );

    const handleMouseMove = useCallback(
        (e: React.MouseEvent) => {
            if (!dragState) return;
            const dx = e.clientX - dragState.startX;
            const dy = e.clientY - dragState.startY;
            const nowDragging = dragState.isDragging || Math.abs(dx) + Math.abs(dy) > 5;

            setDragState((prev) =>
                prev
                    ? { ...prev, currentX: e.clientX, currentY: e.clientY, isDragging: nowDragging }
                    : null
            );

            if (nowDragging) {
                const el = document.elementFromPoint(e.clientX, e.clientY);
                if (isAssistantMode) {
                    const projectEl = el?.closest("[data-project-id]");
                    const noProjectEl = el?.closest("[data-no-project-zone]");
                    setDragOverProjectId(projectEl?.getAttribute("data-project-id") ?? null);
                    setDragOverNoProject(!!noProjectEl && !projectEl);
                } else {
                    const folderEl = el?.closest("[data-folder-id]");
                    const noFolderEl = el?.closest("[data-no-folder-zone]");
                    setDragOverFolderId(folderEl?.getAttribute("data-folder-id") ?? null);
                    setDragOverNoFolder(!!noFolderEl && !folderEl);
                }
            }
        },
        [dragState, isAssistantMode]
    );

    const handleMouseUp = useCallback(
        (e: React.MouseEvent) => {
            if (!dragState) return;
            if (!dragState.isDragging) {
                setDragState(null);
                return;
            }
            const el = document.elementFromPoint(e.clientX, e.clientY);
            if (isAssistantMode) {
                const projectEl = el?.closest("[data-project-id]");
                const noProjectEl = el?.closest("[data-no-project-zone]");
                if (projectEl) {
                    const projectId = projectEl.getAttribute("data-project-id");
                    if (projectId) assignChatToProject(dragState.chatId, projectId);
                } else if (noProjectEl) {
                    removeChatFromProject(dragState.chatId);
                }
                setDragOverProjectId(null);
                setDragOverNoProject(false);
            } else {
                const folderEl = el?.closest("[data-folder-id]");
                const noFolderEl = el?.closest("[data-no-folder-zone]");
                if (folderEl) {
                    const folderId = folderEl.getAttribute("data-folder-id");
                    if (folderId) moveChatToFolder(dragState.chatId, folderId);
                } else if (noFolderEl) {
                    moveChatToFolder(dragState.chatId, null);
                }
                setDragOverFolderId(null);
                setDragOverNoFolder(false);
            }
            setDragState(null);
        },
        [dragState, isAssistantMode, moveChatToFolder, assignChatToProject, removeChatFromProject]
    );

    const handleMouseLeave = useCallback(() => {
        if (dragState?.isDragging) {
            setDragState(null);
            setDragOverFolderId(null);
            setDragOverNoFolder(false);
            setDragOverProjectId(null);
            setDragOverNoProject(false);
        }
    }, [dragState]);

    return (
        <Box
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseLeave}
            style={{
                width: compact ? 60 : width,
                flexShrink: 0,
                minWidth: compact ? 60 : width,
                height: "100%",
                display: "flex",
                flexDirection: "column",
                borderRight:
                    "1px solid var(--mantine-color-default-border)",
                ...style,
            }}
        >
            {compact && (
                <Stack gap={4} align="center" px={4} py="xs" style={{ flex: 1 }}>
                    <Popover
                        position="right-start"
                        width={260}
                        opened={chatsPopoverOpened}
                        onChange={setChatsPopoverOpened}
                    >
                        <Popover.Target>
                            <Tooltip label={t("sidebar.chats")} position="right">
                                <ActionIcon
                                    variant="subtle"
                                    size="lg"
                                    onClick={() => setChatsPopoverOpened((o) => !o)}
                                >
                                    <IconMessages size={20} stroke={1.5} />
                                </ActionIcon>
                            </Tooltip>
                        </Popover.Target>
                        <Popover.Dropdown>{popoverContent}</Popover.Dropdown>
                    </Popover>
                </Stack>
            )}

            <CreateFolderModal
                opened={createFolderModalOpen}
                onClose={() => setCreateFolderModalOpen(false)}
            />
            <CreateProjectModal
                opened={createProjectModalOpen}
                onClose={() => setCreateProjectModalOpen(false)}
            />
            {!compact && (
                <Box style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }} p="xs">
                    {mainContent}
                </Box>
            )}

            {!compact && isAssistantMode && (
                <Box p="xs" style={{ borderTop: "1px solid var(--mantine-color-default-border)", flexShrink: 0 }}>
                    <Group gap="xs" justify="center">
                        <Tooltip label={t("assistantDashboard.contextMemories")}>
                            <Badge size="xs" variant="light" color="teal">{agentMemories.length} mem</Badge>
                        </Tooltip>
                        <Tooltip label={t("assistantDashboard.contextKBs")}>
                            <Badge size="xs" variant="light" color="teal">{knowledgeBases.length} KB</Badge>
                        </Tooltip>
                        <Tooltip label={t("assistantDashboard.contextSkills")}>
                            <Badge size="xs" variant="light" color="teal">{skills.length} skills</Badge>
                        </Tooltip>
                    </Group>
                </Box>
            )}

            <Menu
                opened={contextMenu !== null}
                onChange={(opened) => {
                    if (!opened) setContextMenu(null);
                }}
                position="bottom-start"
            >
                <Menu.Target>
                    <div
                        style={{
                            position: "fixed",
                            left: contextMenu?.x ?? 0,
                            top: contextMenu?.y ?? 0,
                            width: 0,
                            height: 0,
                        }}
                        aria-hidden
                    />
                </Menu.Target>
                <Menu.Dropdown>
                    {isAssistantMode ? (
                        <>
                            <Menu.Label>{t("sidebar.moveToFolder")}</Menu.Label>
                            {activeProjects.map((p) => (
                                <Menu.Item
                                    key={p.id}
                                    leftSection={<IconBriefcase size={14} stroke={1.5} />}
                                    onClick={() => {
                                        if (contextMenu) {
                                            assignChatToProject(contextMenu.chatId, p.id);
                                            setContextMenu(null);
                                        }
                                    }}
                                >
                                    {p.name}
                                </Menu.Item>
                            ))}
                            {contextChat?.projectId != null && (
                                <>
                                    <Menu.Divider />
                                    <Menu.Item
                                        onClick={() => {
                                            if (contextMenu) {
                                                removeChatFromProject(contextMenu.chatId);
                                                setContextMenu(null);
                                            }
                                        }}
                                    >
                                        {t("sidebar.removeFromFolder")}
                                    </Menu.Item>
                                </>
                            )}
                        </>
                    ) : (
                        <>
                            <Menu.Label>{t("sidebar.moveToFolder")}</Menu.Label>
                            {modeFolders.map((f) => (
                                <Menu.Item
                                    key={f.id}
                                    onClick={() => {
                                        if (contextMenu) {
                                            moveChatToFolder(contextMenu.chatId, f.id);
                                            setContextMenu(null);
                                        }
                                    }}
                                >
                                    {f.name}
                                </Menu.Item>
                            ))}
                            {contextChat?.folderId != null && (
                                <>
                                    <Menu.Divider />
                                    <Menu.Item
                                        onClick={() => {
                                            if (contextMenu) {
                                                moveChatToFolder(contextMenu.chatId, null);
                                                setContextMenu(null);
                                            }
                                        }}
                                    >
                                        {t("sidebar.removeFromFolder")}
                                    </Menu.Item>
                                </>
                            )}
                        </>
                    )}
                </Menu.Dropdown>
            </Menu>

            {dragState?.isDragging && (
                <div
                    style={{
                        position: "fixed",
                        left: dragState.currentX + 12,
                        top: dragState.currentY - 10,
                        background: "var(--mantine-color-body)",
                        border: "1px solid var(--mantine-color-default-border)",
                        borderRadius: 4,
                        padding: "4px 8px",
                        fontSize: 12,
                        pointerEvents: "none",
                        zIndex: 1000,
                        boxShadow: "0 2px 8px rgba(0,0,0,0.15)",
                        maxWidth: 200,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                    }}
                >
                    {dragState.chatTitle}
                </div>
            )}
            <ConfirmModal
                opened={deletingChatId !== null}
                onClose={() => setDeletingChatId(null)}
                onConfirm={() => {
                    if (deletingChatId) {
                        deleteChat(deletingChatId);
                        setDeletingChatId(null);
                    }
                }}
                message={t("sidebar.confirmDeleteChat")}
            />
            <ConfirmModal
                opened={deletingFolderId !== null}
                onClose={() => setDeletingFolderId(null)}
                onConfirm={() => {
                    if (deletingFolderId) {
                        deleteFolder(deletingFolderId);
                        setDeletingFolderId(null);
                    }
                }}
                message={t("sidebar.confirmDeleteFolder")}
            />
            <ConfirmModal
                opened={deletingProjectId !== null}
                onClose={() => setDeletingProjectId(null)}
                onConfirm={() => {
                    if (deletingProjectId) {
                        storeDeleteProject(deletingProjectId);
                        setDeletingProjectId(null);
                    }
                }}
                message={t("project.deleteConfirm")}
            />
            <Menu
                opened={projectContextMenu !== null}
                onChange={(opened) => {
                    if (!opened) setProjectContextMenu(null);
                }}
                position="bottom-start"
            >
                <Menu.Target>
                    <div
                        style={{
                            position: "fixed",
                            left: projectContextMenu?.x ?? 0,
                            top: projectContextMenu?.y ?? 0,
                            width: 0,
                            height: 0,
                        }}
                        aria-hidden
                    />
                </Menu.Target>
                <Menu.Dropdown>
                    <Menu.Item
                        leftSection={<IconLayoutBoard size={14} stroke={1.5} />}
                        onClick={() => {
                            if (projectContextMenu) {
                                openProjectDashboard(projectContextMenu.projectId);
                                setProjectContextMenu(null);
                            }
                        }}
                    >
                        {t("project.dashboard.openDashboard")}
                    </Menu.Item>
                    <Menu.Divider />
                    <Menu.Item
                        leftSection={<IconPencil size={14} stroke={1.5} />}
                        onClick={() => {
                            if (projectContextMenu) {
                                const p = activeProjects.find((pr) => pr.id === projectContextMenu.projectId);
                                if (p) startEditProject(p);
                                setProjectContextMenu(null);
                            }
                        }}
                    >
                        {t("project.rename")}
                    </Menu.Item>
                    <Menu.Item
                        leftSection={<IconTarget size={14} stroke={1.5} />}
                        onClick={() => {
                            if (projectContextMenu) {
                                const p = activeProjects.find((pr) => pr.id === projectContextMenu.projectId);
                                if (p) {
                                    setEditGoalProjectId(p.id);
                                    setEditGoalText(p.goal);
                                }
                                setProjectContextMenu(null);
                            }
                        }}
                    >
                        {t("project.editGoal")}
                    </Menu.Item>
                    <Menu.Divider />
                    <Menu.Item
                        leftSection={<IconCheck size={14} stroke={1.5} />}
                        onClick={() => {
                            if (projectContextMenu) {
                                updateProject(projectContextMenu.projectId, { status: "completed" });
                                setProjectContextMenu(null);
                            }
                        }}
                    >
                        {t("project.complete")}
                    </Menu.Item>
                    <Menu.Item
                        leftSection={<IconArchive size={14} stroke={1.5} />}
                        onClick={() => {
                            if (projectContextMenu) {
                                archiveProject(projectContextMenu.projectId);
                                setProjectContextMenu(null);
                            }
                        }}
                    >
                        {t("project.archive")}
                    </Menu.Item>
                    <Menu.Divider />
                    <Menu.Item
                        color="red"
                        leftSection={<IconTrash size={14} stroke={1.5} />}
                        onClick={() => {
                            if (projectContextMenu) {
                                setDeletingProjectId(projectContextMenu.projectId);
                                setProjectContextMenu(null);
                            }
                        }}
                    >
                        {t("project.delete")}
                    </Menu.Item>
                </Menu.Dropdown>
            </Menu>
            <Modal
                size="sm"
                title={t("project.editGoal")}
                opened={editGoalProjectId !== null}
                onClose={() => { setEditGoalProjectId(null); setEditGoalText(""); }}
            >
                <Stack gap="md">
                    <Textarea
                        placeholder={t("project.goalPlaceholder")}
                        value={editGoalText}
                        onChange={(e) => setEditGoalText(e.currentTarget.value)}
                        minRows={2}
                        maxRows={4}
                        autosize
                        autoFocus
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={() => { setEditGoalProjectId(null); setEditGoalText(""); }}>
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={() => {
                            if (editGoalProjectId) {
                                updateProject(editGoalProjectId, { goal: editGoalText.trim() });
                                setEditGoalProjectId(null);
                                setEditGoalText("");
                            }
                        }}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>
        </Box>
    );
}

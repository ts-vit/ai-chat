import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Box,
    Button,
    Collapse,
    Group,
    Menu,
    NavLink,
    Popover,
    ScrollArea,
    Text,
    TextInput,
    Tooltip,
    useMantineColorScheme,
} from "@mantine/core";
import { ColorSwatch } from "@mantine/core";
import {
    IconChevronDown,
    IconChevronRight,
    IconFolderFilled,
    IconFolderPlus,
    IconMessages,
    IconMoon,
    IconPencil,
    IconPlus,
    IconSearch,
    IconSettings,
    IconSun,
    IconTemplate,
    IconTrash,
} from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { formatRelativeDate } from "../utils/formatDate";
import { ConfirmModal } from "./ConfirmModal";
import type { Chat, Folder } from "../types";

const FOLDER_COLORS = [
    "red",
    "pink",
    "grape",
    "violet",
    "indigo",
    "blue",
    "cyan",
    "teal",
    "green",
    "lime",
    "yellow",
    "orange",
] as const;

interface SidebarProps {
    width?: number;
    style?: React.CSSProperties;
    compact?: boolean;
    onNewChat: () => void;
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
}: {
    chat: Chat;
    activeChatId: string | null;
    onSelect: () => void;
    onDelete: () => void;
    onContextMenu?: (e: React.MouseEvent, chatId: string) => void;
    onMouseDown?: (e: React.MouseEvent) => void;
    isDragging?: boolean;
    isActiveDrag?: boolean;
}) {
    const { t } = useTranslation();
    return (
        <div
            className="chat-item"
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
                                ? "3px solid var(--mantine-color-blue-5)"
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
    const { colorScheme, toggleColorScheme } = useMantineColorScheme();

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
        activeChatId,
        deleteChat,
        setActiveChat,
        setView,
        createFolder,
        updateFolder,
        deleteFolder,
        moveChatToFolder,
    } = useChatStore();

    const [deletingChatId, setDeletingChatId] = useState<string | null>(null);
    const [deletingFolderId, setDeletingFolderId] = useState<string | null>(null);
    const [chatsPopoverOpened, setChatsPopoverOpened] = useState(false);
    const [contextMenu, setContextMenu] = useState<{
        chatId: string;
        x: number;
        y: number;
    } | null>(null);
    const [expandedFolderIds, setExpandedFolderIds] = useState<Set<string>>(new Set());
    const [showCreateFolder, setShowCreateFolder] = useState(false);
    const [newFolderName, setNewFolderName] = useState("");
    const [newFolderColor, setNewFolderColor] = useState<string | null>(null);
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

    const toggleFolder = useCallback((folderId: string) => {
        setExpandedFolderIds((prev) => {
            const next = new Set(prev);
            if (next.has(folderId)) next.delete(folderId);
            else next.add(folderId);
            return next;
        });
    }, []);

    const handleCreateFolder = useCallback(() => {
        const name = newFolderName.trim();
        if (name) {
            createFolder(name, newFolderColor);
            setNewFolderName("");
            setNewFolderColor(null);
            setShowCreateFolder(false);
        }
    }, [newFolderName, newFolderColor, createFolder]);

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
        (folderId: string) => chats.filter((c) => c.folderId === folderId),
        [chats]
    );
    const chatsWithoutFolder = chats.filter((c) => !c.folderId);

    const folderColorVar = (color: string | null) =>
        color ? `var(--mantine-color-${color}-5)` : "var(--mantine-color-gray-5)";

    const contextChat = contextMenu ? chats.find((c) => c.id === contextMenu.chatId) : null;

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
            />
        ));

    const renderFolderList = () => (
        <>
            {folders.map((folder) => {
                const count = chatsInFolder(folder.id).length;
                const isExpanded = expandedFolderIds.has(folder.id);
                const isEditing = editingFolderId === folder.id;

                const isDragOver = dragOverFolderId === folder.id;

                return (
                    <Box key={folder.id} mb="xs">
                        <div
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
                                    <Text size="xs" c="dimmed">
                                        {count}
                                    </Text>
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

    const noFolderDropBg =
        colorScheme === "dark"
            ? "var(--mantine-color-dark-6)"
            : "var(--mantine-color-gray-1)";

    const mainContent = (
        <div style={{ flex: 1, overflowY: "auto" }}>
            {!compact && renderFolderList()}
            {!compact && <Box mb="xs" />}
            {!compact ? (
                <Box
                    data-no-folder-zone
                    style={{
                        borderRadius: 4,
                        padding: "2px 4px",
                        minHeight: 8,
                        ...(dragOverNoFolder && { background: noFolderDropBg }),
                    }}
                >
                    {renderChatList(chatsWithoutFolder, () => {}, true)}
                </Box>
            ) : (
                renderChatList(chatsWithoutFolder, () => {})
            )}
        </div>
    );

    const popoverContent = (
        <ScrollArea mah={400} type="scroll">
            {folders.map((folder) => {
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
        </ScrollArea>
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
                const folderEl = el?.closest("[data-folder-id]");
                const noFolderEl = el?.closest("[data-no-folder-zone]");
                setDragOverFolderId(folderEl?.getAttribute("data-folder-id") ?? null);
                setDragOverNoFolder(!!noFolderEl && !folderEl);
            }
        },
        [dragState]
    );

    const handleMouseUp = useCallback(
        (e: React.MouseEvent) => {
            if (!dragState) return;
            if (!dragState.isDragging) {
                setDragState(null);
                return;
            }
            const el = document.elementFromPoint(e.clientX, e.clientY);
            const folderEl = el?.closest("[data-folder-id]");
            const noFolderEl = el?.closest("[data-no-folder-zone]");
            if (folderEl) {
                const folderId = folderEl.getAttribute("data-folder-id");
                if (folderId) moveChatToFolder(dragState.chatId, folderId);
            } else if (noFolderEl) {
                moveChatToFolder(dragState.chatId, null);
            }
            setDragState(null);
            setDragOverFolderId(null);
            setDragOverNoFolder(false);
        },
        [dragState, moveChatToFolder]
    );

    const handleMouseLeave = useCallback(() => {
        if (dragState?.isDragging) {
            setDragState(null);
            setDragOverFolderId(null);
            setDragOverNoFolder(false);
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
                height: "100vh",
                display: "flex",
                flexDirection: "column",
                borderRight:
                    "1px solid var(--mantine-color-default-border)",
                ...style,
            }}
        >
            <Box
                p={compact ? "xs" : "md"}
                style={
                    compact
                        ? { display: "flex", justifyContent: "center" }
                        : undefined
                }
            >
                {compact ? (
                    <Tooltip label={t("sidebar.newChat")}>
                        <ActionIcon
                            size="lg"
                            radius="xl"
                            variant="filled"
                            onClick={onNewChat}
                            aria-label={t("sidebar.newChat")}
                        >
                            <IconPlus size={18} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                ) : (
                    <Group gap="xs" wrap="nowrap">
                        <Button
                            style={{ flex: 1 }}
                            onClick={onNewChat}
                        >
                            + {t("sidebar.newChat")}
                        </Button>
                        <Tooltip label={t("sidebar.newFolder")}>
                            <ActionIcon
                                size="lg"
                                variant="subtle"
                                onClick={() => setShowCreateFolder(true)}
                                aria-label={t("sidebar.newFolder")}
                            >
                                <IconFolderPlus size={18} stroke={1.5} />
                            </ActionIcon>
                        </Tooltip>
                    </Group>
                )}
            </Box>

            {!compact && showCreateFolder && (
                <Box px="md" pb="xs">
                    <TextInput
                        size="xs"
                        placeholder={t("sidebar.folderName")}
                        value={newFolderName}
                        onChange={(e) => setNewFolderName(e.currentTarget.value)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") handleCreateFolder();
                            if (e.key === "Escape") {
                                setShowCreateFolder(false);
                                setNewFolderName("");
                                setNewFolderColor(null);
                            }
                        }}
                        autoFocus
                    />
                    <Group gap={4} mt={4}>
                        {FOLDER_COLORS.map((c) => (
                            <ColorSwatch
                                key={c}
                                color={`var(--mantine-color-${c}-5)`}
                                size={16}
                                onClick={() => setNewFolderColor(c)}
                                style={{
                                    cursor: "pointer",
                                    border:
                                        newFolderColor === c
                                            ? "2px solid var(--mantine-color-default-border)"
                                            : undefined,
                                }}
                            />
                        ))}
                    </Group>
                </Box>
            )}

            {compact ? (
                <Box
                    style={{
                        flex: 1,
                        display: "flex",
                        flexDirection: "column",
                        minHeight: 0,
                    }}
                    p="xs"
                >
                    <Box style={{ display: "flex", justifyContent: "center" }}>
                        <Popover
                            position="right-start"
                            width={260}
                            opened={chatsPopoverOpened}
                            onChange={setChatsPopoverOpened}
                        >
                            <Popover.Target>
                                <Tooltip label={t("sidebar.chats")}>
                                    <ActionIcon
                                        size="lg"
                                        radius="xl"
                                        variant="subtle"
                                        onClick={() =>
                                            setChatsPopoverOpened((o) => !o)
                                        }
                                        aria-label={t("sidebar.chats")}
                                    >
                                        <IconMessages size={18} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            </Popover.Target>
                            <Popover.Dropdown>{popoverContent}</Popover.Dropdown>
                        </Popover>
                    </Box>
                    <Box style={{ flex: 1 }} />
                </Box>
            ) : (
                <Box style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }} p="xs">
                    {mainContent}
                </Box>
            )}

            <Box
                p="md"
                style={{
                    borderTop:
                        "1px solid var(--mantine-color-default-border)",
                }}
            >
                <Group justify="center" gap="md">
                    <Tooltip label={t("sidebar.toggleTheme")}>
                        <ActionIcon
                            size="lg"
                            variant="subtle"
                            onClick={() => toggleColorScheme()}
                            aria-label={
                                colorScheme === "dark"
                                    ? t("sidebar.themeLight")
                                    : t("sidebar.themeDark")
                            }
                        >
                            {colorScheme === "dark" ? (
                                <IconSun size={18} stroke={1.5} />
                            ) : (
                                <IconMoon size={18} stroke={1.5} />
                            )}
                        </ActionIcon>
                    </Tooltip>
                    <Tooltip label={t("common.search")}>
                        <ActionIcon
                            size="lg"
                            variant="subtle"
                            onClick={() => setView("search")}
                            aria-label={t("common.search")}
                        >
                            <IconSearch size={18} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                    <Tooltip label={t("sidebar.snippets")}>
                        <ActionIcon
                            size="lg"
                            variant="subtle"
                            onClick={() => setView("snippets")}
                        >
                            <IconTemplate size={18} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                    <Tooltip label={t("sidebar.settings")}>
                        <ActionIcon
                            size="lg"
                            variant="subtle"
                            onClick={() => setView("settings")}
                        >
                            <IconSettings size={18} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Group>
            </Box>

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
                    <Menu.Label>{t("sidebar.moveToFolder")}</Menu.Label>
                    {folders.map((f) => (
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
        </Box>
    );
}

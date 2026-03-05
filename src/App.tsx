import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Box } from "@mantine/core";
import { AppHeader } from "./components/AppHeader";
import { Sidebar } from "./components/Sidebar";
import { ChatArea } from "./components/ChatArea";
import { CompareView } from "./components/CompareView";
import { ConfirmModal } from "./components/ConfirmModal";
import { NavigationSidebar } from "./components/NavigationSidebar";
import { ProviderSelectModal } from "./components/ProviderSelectModal";
import { ResizeHandle } from "./components/ResizeHandle";
import { SearchPage } from "./components/SearchPage";
import { SettingsPage } from "./components/SettingsPage";
import { SnippetsPage } from "./components/SnippetsPage";
import { useChatStore } from "./store/chatStore";
import { useAppHotkeys } from "./hooks/useHotkeys";
import { useWindowSize } from "./hooks/useWindowSize";

function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
}

function App() {
    const { t } = useTranslation();
    const currentView = useChatStore((s) => s.currentView);
    const chats = useChatStore((s) => s.chats);
    const activeChatId = useChatStore((s) => s.activeChatId);
    const setView = useChatStore((s) => s.setView);
    const setActiveChat = useChatStore((s) => s.setActiveChat);
    const deleteChat = useChatStore((s) => s.deleteChat);
    const loadSettings = useChatStore((s) => s.loadSettings);
    const loadChats = useChatStore((s) => s.loadChats);
    const loadComparisons = useChatStore((s) => s.loadComparisons);
    const loadFolders = useChatStore((s) => s.loadFolders);
    const loadPresets = useChatStore((s) => s.loadPresets);
    const loadCategories = useChatStore((s) => s.loadCategories);
    const loadSnippets = useChatStore((s) => s.loadSnippets);
    const loadCustomProviders = useChatStore((s) => s.loadCustomProviders);
    const loadBalance = useChatStore((s) => s.loadBalance);
    const createChat = useChatStore((s) => s.createChat);

    const { isCompact, isNarrow, isVeryNarrow } = useWindowSize();
    const [leftSidebarWidth, setLeftSidebarWidth] = useState(260);
    const [rightSidebarWidth, setRightSidebarWidth] = useState(270);
    const [leftSidebarOpen, setLeftSidebarOpen] = useState(true);
    const [rightSidebarOpen, setRightSidebarOpen] = useState(false);
    const [providerModalOpened, setProviderModalOpened] = useState(false);
    const [hotkeyDeleteConfirmOpen, setHotkeyDeleteConfirmOpen] = useState(false);
    const messageInputRef = useRef<HTMLTextAreaElement | null>(null);

    const handleNewChat = useCallback(() => {
        setProviderModalOpened(true);
    }, []);

    const handleProviderConfirm = useCallback(
        (providerId: string, model: string, isImageModel: boolean) => {
            createChat(providerId, model, isImageModel);
            setProviderModalOpened(false);
        },
        [createChat]
    );

    useEffect(() => {
        if (isCompact) setRightSidebarOpen(false);
    }, [isCompact]);

    useEffect(() => {
        loadSettings().then(() => loadBalance());
        loadChats();
        loadComparisons();
        loadFolders();
        loadPresets();
        loadCategories();
        loadSnippets();
        loadCustomProviders();
    }, [loadSettings, loadChats, loadComparisons, loadFolders, loadPresets, loadCategories, loadSnippets, loadCustomProviders, loadBalance]);

    useEffect(() => {
        useChatStore.getState().initOllamaPullListeners();
    }, []);

    useAppHotkeys({
        messageInputRef,
        currentView,
        chats,
        activeChatId,
        onNewChat: handleNewChat,
        onSearch: () => setView("search"),
        onSettings: () => setView("settings"),
        onRequestDeleteChat: () => setHotkeyDeleteConfirmOpen(true),
        setActiveChat: (id) => {
            void setActiveChat(id);
        },
        onEscape: () => setView("chat"),
    });

    const handleHotkeyDeleteConfirm = useCallback(() => {
        if (activeChatId) {
            void deleteChat(activeChatId);
        }
        setHotkeyDeleteConfirmOpen(false);
    }, [activeChatId, deleteChat]);

    const effectiveLeftWidth = isNarrow ? 60 : leftSidebarWidth;
    const activeChat = chats.find((c) => c.id === activeChatId) ?? undefined;
    const comparisons = useChatStore((s) => s.comparisons);
    const activeComparisonId = useChatStore((s) => s.activeComparisonId);
    const activeComparison = comparisons.find((c) => c.id === activeComparisonId) ?? null;

    if (currentView === "settings") {
        return <SettingsPage />;
    }
    if (currentView === "snippets") {
        return <SnippetsPage />;
    }
    if (currentView === "search") {
        return <SearchPage />;
    }
    if (currentView === "compare") {
        return (
            <Box
                style={{
                    display: "flex",
                    flexDirection: "column",
                    height: "100vh",
                    overflow: "hidden",
                }}
            >
                <AppHeader
                    leftSidebarOpen={leftSidebarOpen}
                    rightSidebarOpen={rightSidebarOpen}
                    onToggleLeftSidebar={() => setLeftSidebarOpen((o) => !o)}
                    onToggleRightSidebar={() => setRightSidebarOpen((o) => !o)}
                    activeChat={undefined}
                    activeComparison={activeComparison}
                    effectiveLeftWidth={effectiveLeftWidth}
                    isNarrow={isNarrow}
                    isVeryNarrow={isVeryNarrow}
                />
                <Box
                    style={{
                        display: "flex",
                        flex: 1,
                        minHeight: 0,
                        overflow: "hidden",
                    }}
                >
                    {leftSidebarOpen && (
                        <>
                            <Sidebar
                                style={{ width: effectiveLeftWidth }}
                                compact={isNarrow}
                                onNewChat={handleNewChat}
                            />
                            {!isNarrow && (
                                <ResizeHandle
                                    onResize={(delta) =>
                                        setLeftSidebarWidth((w) =>
                                            clamp(w + delta, 200, 400)
                                        )
                                    }
                                />
                            )}
                        </>
                    )}
                    <CompareView />
                </Box>
                <ProviderSelectModal
                    opened={providerModalOpened}
                    onClose={() => setProviderModalOpened(false)}
                    onConfirm={handleProviderConfirm}
                />
            </Box>
        );
    }

    return (
        <Box
            style={{
                display: "flex",
                flexDirection: "column",
                height: "100vh",
                overflow: "hidden",
            }}
        >
            <ProviderSelectModal
                opened={providerModalOpened}
                onClose={() => setProviderModalOpened(false)}
                onConfirm={handleProviderConfirm}
            />
            <ConfirmModal
                opened={hotkeyDeleteConfirmOpen}
                onClose={() => setHotkeyDeleteConfirmOpen(false)}
                onConfirm={handleHotkeyDeleteConfirm}
                message={t("confirm.deleteChatMessage")}
            />
            <AppHeader
                leftSidebarOpen={leftSidebarOpen}
                rightSidebarOpen={rightSidebarOpen}
                onToggleLeftSidebar={() => setLeftSidebarOpen((o) => !o)}
                onToggleRightSidebar={() => setRightSidebarOpen((o) => !o)}
                activeChat={activeChat}
                effectiveLeftWidth={effectiveLeftWidth}
                isNarrow={isNarrow}
                isVeryNarrow={isVeryNarrow}
            />
            <Box
                style={{
                    display: "flex",
                    flex: 1,
                    minHeight: 0,
                    overflow: "hidden",
                }}
            >
                {leftSidebarOpen && (
                    <>
                        <Sidebar
                            style={{ width: effectiveLeftWidth }}
                            compact={isNarrow}
                            onNewChat={handleNewChat}
                        />
                        {!isNarrow && (
                            <ResizeHandle
                                onResize={(delta) =>
                                    setLeftSidebarWidth((w) =>
                                        clamp(w + delta, 200, 400)
                                    )
                                }
                            />
                        )}
                    </>
                )}
                <ChatArea
                    compact={isCompact}
                    hideStats={isVeryNarrow}
                    onNewChat={handleNewChat}
                    messageInputRef={messageInputRef}
                />
                {rightSidebarOpen && (
                    <>
                        <ResizeHandle
                            onResize={(delta) =>
                                setRightSidebarWidth((w) =>
                                    clamp(w - delta, 240, 400)
                                )
                            }
                        />
                        <Box
                            style={{
                                width: rightSidebarWidth,
                                flexShrink: 0,
                                display: "flex",
                                flexDirection: "column",
                                minWidth: 0,
                            }}
                        >
                            <NavigationSidebar
                                style={{ width: "100%", minWidth: 0 }}
                            />
                        </Box>
                    </>
                )}
            </Box>
        </Box>
    );
}

export default App;
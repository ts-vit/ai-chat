import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Box } from "@mantine/core";
import { AppHeader } from "./components/AppHeader";
import { Sidebar } from "./components/Sidebar";
import { ChatArea } from "./components/ChatArea";
import { ComparisonsPage } from "./components/ComparisonsPage";
import { CompareView } from "./components/CompareView";
import { ConfirmModal } from "./components/ConfirmModal";
import { NavigationSidebar } from "./components/NavigationSidebar";
import { ProviderSelectModal } from "./components/ProviderSelectModal";
import { ResizeHandle } from "./components/ResizeHandle";
import { SearchPage } from "./components/SearchPage";
import { SettingsPage } from "./components/SettingsPage";
import { SnippetsPage } from "./components/SnippetsPage";
import MemoryPage from "./components/MemoryPage";
import { PromptLibraryPage } from "./components/PromptLibraryPage";
import { SkillsPage } from "./components/SkillsPage";
import { PlansPage } from "./components/PlansPage";
import { ProjectDashboardPage } from "./components/ProjectDashboardPage";
import { SchedulerPage } from "./components/SchedulerPage";
import { KnowledgeBasesPage } from "./components/KnowledgeBasesPage";
import NotebookPage from "./components/NotebookPage";
import NotebookWorkspace from "./components/NotebookWorkspace";
import { PlanPanel } from "./components/PlanPanel";
import { WorkspacePanel } from "./components/WorkspacePanel";
import { TerminalPanel } from "./components/TerminalPanel";
import { useChatStore } from "./store/chatStore";
import { useAppHotkeys } from "./hooks/useHotkeys";
import { useWindowSize } from "./hooks/useWindowSize";
import { listen } from "@tauri-apps/api/event";
import { notify } from "./utils/notify";

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
    const loadWelcomeSnippets = useChatStore((s) => s.loadWelcomeSnippets);
    const loadCustomProviders = useChatStore((s) => s.loadCustomProviders);
    const loadBalance = useChatStore((s) => s.loadBalance);
    const loadModeSettings = useChatStore((s) => s.loadModeSettings);
    const loadSkills = useChatStore((s) => s.loadSkills);
    const loadProjects = useChatStore((s) => s.loadProjects);
    const createChat = useChatStore((s) => s.createChat);
    const showPlanPanel = useChatStore((s) => s.showPlanPanel);
    const activePlan = useChatStore((s) => s.activePlan);
    const showWorkspacePanel = useChatStore((s) => s.showWorkspacePanel);
    const activeMode = useChatStore((s) => s.activeMode);
    const activeNotebookId = useChatStore((s) => s.activeNotebookId);

    const { isCompact, isNarrow, isVeryNarrow } = useWindowSize();
    const [leftSidebarWidth, setLeftSidebarWidth] = useState(260);
    const [rightSidebarWidth, setRightSidebarWidth] = useState(270);
    const [leftSidebarOpen, setLeftSidebarOpen] = useState(true);
    const [rightSidebarOpen, setRightSidebarOpen] = useState(false);
    const [providerModalOpened, setProviderModalOpened] = useState(false);
    const [hotkeyDeleteConfirmOpen, setHotkeyDeleteConfirmOpen] = useState(false);
    const [terminalOpen, setTerminalOpen] = useState(false);
    const [terminalHeight, setTerminalHeight] = useState(250);
    const [terminalPosition, setTerminalPosition] = useState<"bottom" | "right">("bottom");
    const [terminalWidth, setTerminalWidth] = useState(400);
    const settings = useChatStore((s) => s.settings);
    const messageInputRef = useRef<HTMLTextAreaElement | null>(null);

    const toggleTerminal = useCallback(() => {
        setTerminalOpen((prev) => !prev);
    }, []);

    const [pendingProjectId, setPendingProjectId] = useState<string | undefined>(undefined);

    const handleNewChat = useCallback((projectId?: string) => {
        setPendingProjectId(projectId);
        setProviderModalOpened(true);
    }, []);

    const handleProviderConfirm = useCallback(
        (providerId: string, model: string, isImageModel: boolean) => {
            createChat(providerId, model, isImageModel, undefined, pendingProjectId);
            setProviderModalOpened(false);
            setPendingProjectId(undefined);
        },
        [createChat, pendingProjectId]
    );

    useEffect(() => {
        if (isCompact) setRightSidebarOpen(false);
    }, [isCompact]);

    useEffect(() => {
        loadSettings().then(() => loadBalance());
        loadChats().then(() => {
            const id = useChatStore.getState().activeChatId;
            if (id) useChatStore.getState().setActiveChat(id);
        });
        loadComparisons();
        loadFolders();
        loadProjects();
        loadPresets();
        loadCategories();
        loadSnippets();
        loadWelcomeSnippets();
        loadCustomProviders();
        loadModeSettings();
        loadSkills();
    }, [loadSettings, loadChats, loadComparisons, loadFolders, loadProjects, loadPresets, loadCategories, loadSnippets, loadWelcomeSnippets, loadCustomProviders, loadBalance, loadModeSettings, loadSkills]);

    useEffect(() => {
        useChatStore.getState().initOllamaPullListeners();
        useChatStore.getState().initTtsListeners();
        useChatStore.getState().initPlanListeners();
        useChatStore.getState().initWorkspaceListeners();
        useChatStore.getState().initSubAgentListeners();

        // Scheduler event listeners
        listen<{ taskId: string; taskName: string; resultPreview: string }>(
            "scheduler-task-completed",
            (event) => {
                notify.success(t("scheduler.taskCompleted", { name: event.payload.taskName }));
                useChatStore.getState().loadScheduledTasks();
            }
        );
        listen<{ taskId: string; taskName: string; error: string }>(
            "scheduler-task-failed",
            (event) => {
                notify.error(t("scheduler.taskFailed", { name: event.payload.taskName }));
                useChatStore.getState().loadScheduledTasks();
            }
        );
        listen("scheduler-task-started", () => {
            useChatStore.getState().loadScheduledTasks();
        });
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
        onEscape: () => setView(currentView === "compare" ? "comparisons" : "chat"),
        onToggleTerminal: toggleTerminal,
    });

    const handleHotkeyDeleteConfirm = useCallback(() => {
        if (activeChatId) {
            void deleteChat(activeChatId);
        }
        setHotkeyDeleteConfirmOpen(false);
    }, [activeChatId, deleteChat]);

    const sidebarCompact = isNarrow || !leftSidebarOpen;
    const effectiveLeftWidth = sidebarCompact ? 60 : leftSidebarWidth;
    const activeChat = chats.find((c) => c.id === activeChatId) ?? undefined;
    const comparisons = useChatStore((s) => s.comparisons);
    const activeComparisonId = useChatStore((s) => s.activeComparisonId);
    const activeComparison = comparisons.find((c) => c.id === activeComparisonId) ?? null;

    const terminalProps = {
        position: terminalPosition,
        onPositionChange: setTerminalPosition,
        onClose: () => setTerminalOpen(false),
        fontSize: settings.terminalFontSize ?? 13,
        shell: settings.terminalShell,
    };

    const terminalBottom = terminalOpen && terminalPosition === "bottom" && (
        <>
            <ResizeHandle
                direction="horizontal"
                onResize={(delta) =>
                    setTerminalHeight((h) =>
                        clamp(h - delta, 100, Math.round(window.innerHeight * 0.6))
                    )
                }
            />
            <TerminalPanel height={terminalHeight} {...terminalProps} />
        </>
    );

    const terminalRight = terminalOpen && terminalPosition === "right" && (
        <>
            <ResizeHandle
                direction="vertical"
                onResize={(delta) =>
                    setTerminalWidth((w) =>
                        clamp(w - delta, 200, Math.round(window.innerWidth * 0.6))
                    )
                }
            />
            <TerminalPanel width={terminalWidth} {...terminalProps} />
        </>
    );

    // Notebook mode: full-page, no sidebar
    if (activeMode === "notebook") {
        return (
            <Box style={{ display: "flex", flexDirection: "column", height: "100vh", overflow: "hidden" }}>
                <AppHeader
                    activeChat={undefined}
                    onNewChat={() => {}}
                    onToggleSidebar={() => {}}
                    leftSidebarOpen={false}
                    terminalOpen={terminalOpen}
                    onToggleTerminal={toggleTerminal}
                />
                <Box style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
                    <Box style={{ flex: 1, overflow: "hidden" }}>
                        {activeNotebookId ? <NotebookWorkspace /> : <NotebookPage />}
                    </Box>
                    {terminalRight}
                </Box>
                {terminalBottom}
            </Box>
        );
    }

    if (currentView === "settings" || currentView === "snippets" || currentView === "search" || currentView === "comparisons" || currentView === "promptLibrary" || currentView === "memory" || currentView === "skills" || currentView === "plans" || currentView === "projectDashboard" || currentView === "scheduler" || currentView === "knowledgeBases") {
        const PageComponent = {
            settings: SettingsPage,
            snippets: SnippetsPage,
            search: SearchPage,
            comparisons: ComparisonsPage,
            promptLibrary: PromptLibraryPage,
            memory: MemoryPage,
            skills: SkillsPage,
            plans: PlansPage,
            projectDashboard: ProjectDashboardPage,
            scheduler: SchedulerPage,
            knowledgeBases: KnowledgeBasesPage,
        }[currentView];
        return (
            <Box style={{ display: "flex", flexDirection: "column", height: "100vh", overflow: "hidden" }}>
                <Box style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
                    <Box style={{ flex: 1, overflow: "hidden" }}><PageComponent /></Box>
                    {terminalRight}
                </Box>
                {terminalBottom}
            </Box>
        );
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
                    activeChat={undefined}
                    activeComparison={activeComparison}
                    onNewChat={() => handleNewChat()}
                    onToggleSidebar={() => setLeftSidebarOpen((o) => !o)}
                    leftSidebarOpen={leftSidebarOpen}
                    terminalOpen={terminalOpen}
                    onToggleTerminal={toggleTerminal}
                />
                <Box
                    style={{
                        display: "flex",
                        flex: 1,
                        minHeight: 0,
                        overflow: "hidden",
                    }}
                >
                    <Sidebar
                        style={{ width: effectiveLeftWidth }}
                        compact={sidebarCompact}
                        onNewChat={handleNewChat}
                    />
                    {!sidebarCompact && (
                        <ResizeHandle
                            onResize={(delta) =>
                                setLeftSidebarWidth((w) =>
                                    clamp(w + delta, 200, 400)
                                )
                            }
                        />
                    )}
                    <Box style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, minWidth: 0, overflow: "hidden" }}>
                        <Box style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
                            <CompareView />
                            {terminalRight}
                        </Box>
                        {terminalBottom}
                    </Box>
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
                activeChat={activeChat}
                onNewChat={() => handleNewChat()}
                onToggleSidebar={() => setLeftSidebarOpen((o) => !o)}
                leftSidebarOpen={leftSidebarOpen}
                terminalOpen={terminalOpen}
                onToggleTerminal={toggleTerminal}
            />
            <Box style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
                <Sidebar
                    style={{ width: effectiveLeftWidth }}
                    compact={sidebarCompact}
                    onNewChat={handleNewChat}
                />
                {!sidebarCompact && (
                    <ResizeHandle
                        onResize={(delta) =>
                            setLeftSidebarWidth((w) =>
                                clamp(w + delta, 200, 400)
                            )
                        }
                    />
                )}
                <Box style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, minWidth: 0, overflow: "hidden" }}>
                    <Box
                        style={{
                            display: "flex",
                            flex: 1,
                            minHeight: 0,
                            overflow: "hidden",
                        }}
                    >
                        <ChatArea
                            compact={isCompact}
                            hideStats={isVeryNarrow}
                            onNewChat={handleNewChat}
                            messageInputRef={messageInputRef}
                        />
                        {rightSidebarOpen && (
                            <ResizeHandle
                                onResize={(delta) =>
                                    setRightSidebarWidth((w) =>
                                        clamp(w - delta, 240, 400)
                                    )
                                }
                            />
                        )}
                        {showWorkspacePanel ? (
                            <WorkspacePanel />
                        ) : showPlanPanel && activePlan ? (
                            <PlanPanel />
                        ) : (
                            <NavigationSidebar
                                compact={!rightSidebarOpen}
                                onToggle={() => setRightSidebarOpen((o) => !o)}
                                width={rightSidebarOpen ? rightSidebarWidth : undefined}
                                style={rightSidebarOpen ? { minWidth: 0 } : undefined}
                            />
                        )}
                        {terminalRight}
                    </Box>
                    {terminalBottom}
                </Box>
            </Box>
        </Box>
    );
}

export default App;
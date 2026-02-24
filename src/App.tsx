import { useEffect, useState } from "react";
import { Box } from "@mantine/core";
import { Sidebar } from "./components/Sidebar";
import { ChatArea } from "./components/ChatArea";
import { NavigationSidebar } from "./components/NavigationSidebar";
import { ResizeHandle } from "./components/ResizeHandle";
import { SettingsPage } from "./components/SettingsPage";
import { SnippetsPage } from "./components/SnippetsPage";
import { useChatStore } from "./store/chatStore";
import { useWindowSize } from "./hooks/useWindowSize";

function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
}

function App() {
    const currentView = useChatStore((s) => s.currentView);
    const loadSettings = useChatStore((s) => s.loadSettings);
    const loadChats = useChatStore((s) => s.loadChats);
    const loadPresets = useChatStore((s) => s.loadPresets);
    const loadCategories = useChatStore((s) => s.loadCategories);
    const loadSnippets = useChatStore((s) => s.loadSnippets);
    const loadBalance = useChatStore((s) => s.loadBalance);

    const { isCompact, isNarrow, isVeryNarrow } = useWindowSize();
    const [leftSidebarWidth, setLeftSidebarWidth] = useState(260);
    const [rightSidebarWidth, setRightSidebarWidth] = useState(240);
    const [leftSidebarOpen, setLeftSidebarOpen] = useState(true);
    const [rightSidebarOpen, setRightSidebarOpen] = useState(false);

    useEffect(() => {
        if (isCompact) setRightSidebarOpen(false);
    }, [isCompact]);

    useEffect(() => {
        loadSettings().then(() => loadBalance());
        loadChats();
        loadPresets();
        loadCategories();
        loadSnippets();
    }, [loadSettings, loadChats, loadPresets, loadCategories, loadSnippets, loadBalance]);

    if (currentView === "settings") {
        return <SettingsPage />;
    }
    if (currentView === "snippets") {
        return <SnippetsPage />;
    }

    const effectiveLeftWidth = isNarrow ? 60 : leftSidebarWidth;

    return (
        <Box style={{ display: "flex", height: "100vh", overflow: "hidden" }}>
            {leftSidebarOpen && (
                <>
                    <Sidebar style={{ width: effectiveLeftWidth }} compact={isNarrow} />
                    {!isNarrow && (
                        <ResizeHandle
                            onResize={(delta) =>
                                setLeftSidebarWidth((w) => clamp(w + delta, 200, 400))
                            }
                        />
                    )}
                </>
            )}
            <ChatArea
                onToggleLeftSidebar={() => setLeftSidebarOpen((o) => !o)}
                onToggleRightSidebar={() => setRightSidebarOpen((o) => !o)}
                leftSidebarOpen={leftSidebarOpen}
                rightSidebarOpen={rightSidebarOpen}
                compact={isCompact}
                hideStats={isVeryNarrow}
            />
            {rightSidebarOpen && (
                <>
                    <ResizeHandle
                        onResize={(delta) =>
                            setRightSidebarWidth((w) => clamp(w - delta, 180, 400))
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
                        <NavigationSidebar style={{ width: "100%", minWidth: 0 }} />
                    </Box>
                </>
            )}
        </Box>
    );
}

export default App;
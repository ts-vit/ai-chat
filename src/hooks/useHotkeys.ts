import { useHotkeys as useMantineHotkeys } from "@mantine/hooks";
import type { RefObject } from "react";

export interface UseAppHotkeysOptions {
    messageInputRef: RefObject<HTMLTextAreaElement | null> | null;
    currentView: "chat" | "settings" | "snippets" | "search";
    chats: { id: string }[];
    activeChatId: string | null;
    onNewChat: () => void;
    onSearch: () => void;
    onSettings: () => void;
    onRequestDeleteChat: () => void;
    setActiveChat: (id: string) => void;
    onEscape: () => void;
}

/**
 * Centralized app hotkeys. Uses empty tagsToIgnore so that Ctrl+combos and Escape
 * work even when focus is in the message textarea.
 */
export function useAppHotkeys(options: UseAppHotkeysOptions): void {
    const {
        messageInputRef,
        currentView,
        chats,
        activeChatId,
        onNewChat,
        onSearch,
        onSettings,
        onRequestDeleteChat,
        setActiveChat,
        onEscape,
    } = options;

    useMantineHotkeys(
        [
            [
                "mod+N",
                (e) => {
                    e.preventDefault();
                    onNewChat();
                },
                { preventDefault: true, usePhysicalKeys: true },
            ],
            [
                "mod+/",
                (e) => {
                    e.preventDefault();
                    messageInputRef?.current?.focus();
                },
                { preventDefault: true, usePhysicalKeys: true },
            ],
            [
                "mod+Shift+F",
                (e) => {
                    e.preventDefault();
                    onSearch();
                },
                { preventDefault: true, usePhysicalKeys: true },
            ],
            [
                "mod+Comma",
                (e) => {
                    e.preventDefault();
                    onSettings();
                },
                { preventDefault: true, usePhysicalKeys: true },
            ],
            [
                "mod+Backspace",
                (e) => {
                    e.preventDefault();
                    if (activeChatId) onRequestDeleteChat();
                },
                { preventDefault: true, usePhysicalKeys: true },
            ],
            [
                "mod+ArrowUp",
                (e) => {
                    e.preventDefault();
                    if (chats.length === 0 || !activeChatId) return;
                    const idx = chats.findIndex((c) => c.id === activeChatId);
                    if (idx <= 0) return;
                    setActiveChat(chats[idx - 1].id);
                },
                { preventDefault: true, usePhysicalKeys: true },
            ],
            [
                "mod+ArrowDown",
                (e) => {
                    e.preventDefault();
                    if (chats.length === 0 || !activeChatId) return;
                    const idx = chats.findIndex((c) => c.id === activeChatId);
                    if (idx === -1 || idx >= chats.length - 1) return;
                    setActiveChat(chats[idx + 1].id);
                },
                { preventDefault: true, usePhysicalKeys: true },
            ],
            [
                "Escape",
                () => {
                    if (currentView !== "chat") onEscape();
                },
                { usePhysicalKeys: true },
            ],
        ],
        [] // empty = do not ignore INPUT/TEXTAREA, so Ctrl+combos and Escape work in message input
    );
}

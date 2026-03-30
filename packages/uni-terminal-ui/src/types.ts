import type { Terminal } from "@xterm/xterm";
import type { FitAddon } from "@xterm/addon-fit";
import type { UnlistenFn } from "@tauri-apps/api/event";

export interface TerminalPanelProps {
    height?: number;
    width?: number;
    position: "bottom" | "right";
    onPositionChange: (pos: "bottom" | "right") => void;
    onClose: () => void;
    fontSize?: number;
    shell?: string;
    cwd?: string;
}

export interface TerminalTab {
    id: string;
    title: string;
    terminal: Terminal;
    fitAddon: FitAddon;
    containerDiv: HTMLDivElement;
    unlistenData: UnlistenFn | null;
    unlistenExit: UnlistenFn | null;
    proxyUrl: string | null;
    keydownHandler: ((e: KeyboardEvent) => void) | null;
}

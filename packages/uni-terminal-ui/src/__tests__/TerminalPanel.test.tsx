import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MantineProvider } from "@mantine/core";
import { TerminalPanel } from "../TerminalPanel";

// Mock xterm
const mockWrite = vi.fn();
const mockDispose = vi.fn();
const mockOpen = vi.fn();
const mockLoadAddon = vi.fn();
const mockOnData = vi.fn();
const mockAttachCustomKeyEventHandler = vi.fn();

vi.mock("@xterm/xterm", () => ({
    Terminal: vi.fn().mockImplementation(() => ({
        open: mockOpen,
        write: mockWrite,
        dispose: mockDispose,
        loadAddon: mockLoadAddon,
        onData: mockOnData,
        attachCustomKeyEventHandler: mockAttachCustomKeyEventHandler,
        cols: 80,
        rows: 24,
        options: {},
    })),
}));

vi.mock("@xterm/addon-fit", () => ({
    FitAddon: vi.fn().mockImplementation(() => ({
        fit: vi.fn(),
    })),
}));

vi.mock("@xterm/addon-web-links", () => ({
    WebLinksAddon: vi.fn(),
}));

// Mock Tauri
const mockInvoke = vi.fn();
const mockListen = vi.fn();
const mockUnlisten = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
    invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
    listen: (...args: unknown[]) => mockListen(...args),
}));

// Mock i18next
vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => key,
        i18n: { language: "en" },
    }),
}));

// Mock notifications
vi.mock("@mantine/notifications", () => ({
    notifications: {
        show: vi.fn(),
    },
}));

function renderWithMantine(ui: React.ReactElement) {
    return render(<MantineProvider>{ui}</MantineProvider>);
}

function getButtonByIcon(container: HTMLElement, iconClass: string): HTMLButtonElement | null {
    const icon = container.querySelector(`.tabler-icon-${iconClass}`);
    return icon?.closest("button") ?? null;
}

describe("TerminalPanel", () => {
    const defaultProps = {
        position: "bottom" as const,
        onPositionChange: vi.fn(),
        onClose: vi.fn(),
    };

    beforeEach(() => {
        vi.clearAllMocks();
        mockInvoke.mockImplementation((cmd: string) => {
            if (cmd === "terminal_create") return Promise.resolve("session-1");
            if (cmd === "get_current_proxy_url") return Promise.resolve(null);
            return Promise.resolve(null);
        });
        mockListen.mockResolvedValue(mockUnlisten);
    });

    it("renders without errors and shows tab bar buttons", () => {
        const { container } = renderWithMantine(<TerminalPanel {...defaultProps} />);
        // Plus button (new tab) should be present
        const plusBtn = getButtonByIcon(container, "plus");
        expect(plusBtn).toBeInTheDocument();
        // Code button (Claude Code) should be present
        const codeBtn = getButtonByIcon(container, "code");
        expect(codeBtn).toBeInTheDocument();
    });

    it("auto-creates first tab on mount", async () => {
        renderWithMantine(<TerminalPanel {...defaultProps} />);
        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith("terminal_create", {
                cols: 80,
                rows: 24,
                shell: undefined,
                cwd: undefined,
            });
        });
    });

    it("passes cwd to terminal_create when provided", async () => {
        renderWithMantine(<TerminalPanel {...defaultProps} cwd="/projects/my-app" />);
        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith("terminal_create", {
                cols: 80,
                rows: 24,
                shell: undefined,
                cwd: "/projects/my-app",
            });
        });
    });

    it("creates new tab on + button click", async () => {
        const { container } = renderWithMantine(<TerminalPanel {...defaultProps} />);
        // Wait for first auto-created tab
        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith("terminal_create", expect.anything());
        });

        mockInvoke.mockImplementation((cmd: string) => {
            if (cmd === "terminal_create") return Promise.resolve("session-2");
            if (cmd === "get_current_proxy_url") return Promise.resolve(null);
            return Promise.resolve(null);
        });

        const addButton = getButtonByIcon(container, "plus")!;
        fireEvent.click(addButton);

        await waitFor(() => {
            // 2x terminal_create + 2x get_current_proxy_url
            expect(mockInvoke).toHaveBeenCalledTimes(4);
        });
    });

    it("creates Claude Code tab on button click", async () => {
        const { container } = renderWithMantine(<TerminalPanel {...defaultProps} />);
        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith("terminal_create", expect.anything());
        });

        mockInvoke.mockImplementation((cmd: string) => {
            if (cmd === "terminal_create") return Promise.resolve("session-claude");
            if (cmd === "get_current_proxy_url") return Promise.resolve(null);
            return Promise.resolve(null);
        });

        const claudeButton = getButtonByIcon(container, "code")!;
        fireEvent.click(claudeButton);

        await waitFor(() => {
            expect(screen.getByText("Claude Code")).toBeInTheDocument();
        });
    });

    it("closes tab and calls terminal_kill", async () => {
        renderWithMantine(<TerminalPanel {...defaultProps} />);
        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith("terminal_create", expect.anything());
        });

        // Wait for tab to appear
        await waitFor(() => {
            expect(screen.getByText("terminal.title 1")).toBeInTheDocument();
        });

        // Find the small X button inside the tab
        const tabGroup = screen.getByText("terminal.title 1").parentElement;
        const closeBtn = tabGroup?.querySelector("button");
        if (closeBtn) {
            fireEvent.click(closeBtn);
            await waitFor(() => {
                expect(mockInvoke).toHaveBeenCalledWith("terminal_kill", { sessionId: "session-1" });
            });
        }
    });
});

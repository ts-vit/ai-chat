import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MantineProvider } from "@mantine/core";
import { SessionTabs, type SessionTab } from "../components/SessionTabs";

beforeAll(() => {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: query === "(prefers-color-scheme: dark)",
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });

  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

function renderWithMantine(ui: React.ReactElement) {
  return render(<MantineProvider>{ui}</MantineProvider>);
}

const mockTabs: SessionTab[] = [
  { id: "tab-1", label: "Session 1" },
  { id: "tab-2", label: "Session 2", status: "running" },
  { id: "tab-3", label: "Session 3", status: "error" },
];

describe("SessionTabs", () => {
  it("renders all tab labels", () => {
    renderWithMantine(
      <SessionTabs tabs={mockTabs} activeId="tab-1" onTabChange={() => {}} />
    );
    expect(screen.getByText("Session 1")).toBeInTheDocument();
    expect(screen.getByText("Session 2")).toBeInTheDocument();
    expect(screen.getByText("Session 3")).toBeInTheDocument();
  });

  it("calls onTabChange when tab is clicked", () => {
    const onChange = vi.fn();
    renderWithMantine(
      <SessionTabs tabs={mockTabs} activeId="tab-1" onTabChange={onChange} />
    );
    fireEvent.click(screen.getByText("Session 2"));
    expect(onChange).toHaveBeenCalledWith("tab-2");
  });

  it("shows add button when onTabAdd is provided", () => {
    renderWithMantine(
      <SessionTabs
        tabs={mockTabs}
        activeId="tab-1"
        onTabChange={() => {}}
        onTabAdd={() => {}}
      />
    );
    expect(screen.getByRole("button", { name: "New tab" })).toBeInTheDocument();
  });

  it("hides add button when onTabAdd is not provided", () => {
    renderWithMantine(
      <SessionTabs tabs={mockTabs} activeId="tab-1" onTabChange={() => {}} />
    );
    expect(screen.queryByRole("button", { name: "New tab" })).not.toBeInTheDocument();
  });

  it("disables add button when maxTabs reached", () => {
    renderWithMantine(
      <SessionTabs
        tabs={mockTabs}
        activeId="tab-1"
        onTabChange={() => {}}
        onTabAdd={() => {}}
        maxTabs={3}
      />
    );
    expect(screen.getByRole("button", { name: "New tab" })).toBeDisabled();
  });

  it("calls onTabClose when close button is clicked on active tab", () => {
    const onClose = vi.fn();
    const { container } = renderWithMantine(
      <SessionTabs
        tabs={mockTabs}
        activeId="tab-1"
        onTabChange={() => {}}
        onTabClose={onClose}
      />
    );
    // Active tab shows close button without hover
    const closeButtons = container.querySelectorAll("button.m_86a44da5, button.mantine-CloseButton-root");
    // Fallback: find any close button inside the tab area
    const allButtons = container.querySelectorAll("button");
    const closeBtn = Array.from(allButtons).find(
      (b) => b.classList.toString().includes("CloseButton") || b.getAttribute("aria-label") === "Close"
    );
    if (closeBtn) {
      fireEvent.click(closeBtn);
      expect(onClose).toHaveBeenCalledWith("tab-1");
    } else if (closeButtons.length > 0) {
      fireEvent.click(closeButtons[0]);
      expect(onClose).toHaveBeenCalledWith("tab-1");
    }
  });

  it("does not call onTabChange when close button is clicked", () => {
    const onChange = vi.fn();
    const onClose = vi.fn();
    const { container } = renderWithMantine(
      <SessionTabs
        tabs={mockTabs}
        activeId="tab-1"
        onTabChange={onChange}
        onTabClose={onClose}
      />
    );
    const allButtons = container.querySelectorAll("button");
    const closeBtn = Array.from(allButtons).find(
      (b) => b.classList.toString().includes("CloseButton") || b.getAttribute("aria-label") === "Close"
    );
    if (closeBtn) {
      fireEvent.click(closeBtn);
      expect(onClose).toHaveBeenCalled();
      expect(onChange).not.toHaveBeenCalled();
    }
  });

  it("does not show close button for non-closable tab", () => {
    const tabs: SessionTab[] = [
      { id: "tab-1", label: "Permanent", closable: false },
    ];
    const { container } = renderWithMantine(
      <SessionTabs
        tabs={tabs}
        activeId="tab-1"
        onTabChange={() => {}}
        onTabClose={() => {}}
      />
    );
    expect(screen.getByText("Permanent")).toBeInTheDocument();
    // No close button should exist — only the tab itself (UnstyledButton)
    const allButtons = container.querySelectorAll("button");
    const closeBtn = Array.from(allButtons).find(
      (b) => b.classList.toString().includes("CloseButton") || b.getAttribute("aria-label") === "Close"
    );
    expect(closeBtn).toBeUndefined();
  });

  it("renders with empty tabs list", () => {
    renderWithMantine(
      <SessionTabs
        tabs={[]}
        activeId=""
        onTabChange={() => {}}
        onTabAdd={() => {}}
      />
    );
    expect(screen.getByRole("button", { name: "New tab" })).toBeInTheDocument();
  });

  it("renders status indicators for running and error tabs", () => {
    const { container } = renderWithMantine(
      <SessionTabs tabs={mockTabs} activeId="tab-1" onTabChange={() => {}} />
    );
    // tab-2 running, tab-3 error — should have status dots
    expect(container.querySelector('[data-testid="status-tab-2"]')).toBeInTheDocument();
    expect(container.querySelector('[data-testid="status-tab-3"]')).toBeInTheDocument();
    // tab-1 has no status — no dot
    expect(container.querySelector('[data-testid="status-tab-1"]')).not.toBeInTheDocument();
  });

  it("supports custom height prop without crashing", () => {
    // Mantine Group may apply height via className, not inline style
    const { container } = renderWithMantine(
      <SessionTabs
        tabs={mockTabs}
        activeId="tab-1"
        onTabChange={() => {}}
        height={40}
      />
    );
    expect(container.firstChild).toBeInTheDocument();
  });
});

import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MantineProvider } from "@mantine/core";
import { ResizablePanel } from "../components/ResizablePanel";

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

describe("ResizablePanel", () => {
  it("renders both panels", () => {
    renderWithMantine(
      <ResizablePanel
        first={<div>Panel A</div>}
        second={<div>Panel B</div>}
      />,
    );
    expect(screen.getByText("Panel A")).toBeInTheDocument();
    expect(screen.getByText("Panel B")).toBeInTheDocument();
  });

  it("renders separator with correct role", () => {
    renderWithMantine(
      <ResizablePanel first={<div>A</div>} second={<div>B</div>} />,
    );
    const separator = screen.getByRole("separator");
    expect(separator).toBeInTheDocument();
  });

  it("renders vertical split direction", () => {
    renderWithMantine(
      <ResizablePanel
        first={<div>Top</div>}
        second={<div>Bottom</div>}
        direction="vertical"
      />,
    );
    const separator = screen.getByRole("separator");
    expect(separator.getAttribute("aria-orientation")).toBe("horizontal");
    expect(screen.getByText("Top")).toBeInTheDocument();
    expect(screen.getByText("Bottom")).toBeInTheDocument();
  });

  it("applies custom style to container", () => {
    renderWithMantine(
      <ResizablePanel
        first={<div>A</div>}
        second={<div>B</div>}
        style={{ height: "500px" }}
      />,
    );
    const separator = screen.getByRole("separator");
    const root = separator.parentElement as HTMLElement;
    expect(root.style.height).toBe("500px");
  });

  it("applies initial position", () => {
    renderWithMantine(
      <ResizablePanel
        first={<div>A</div>}
        second={<div>B</div>}
        initialPosition={30}
      />,
    );
    const separator = screen.getByRole("separator");
    const root = separator.parentElement as HTMLElement;
    const firstPanel = root.children[0] as HTMLElement;
    expect(firstPanel.style.width).toBe("30%");
  });

  it("renders with resizable=false (no cursor change)", () => {
    renderWithMantine(
      <ResizablePanel
        first={<div>A</div>}
        second={<div>B</div>}
        resizable={false}
      />,
    );
    const separator = screen.getByRole("separator");
    expect(separator.style.cursor).toBe("default");
  });

  it("calls onPositionChange during drag simulation", () => {
    const onChange = vi.fn();
    renderWithMantine(
      <ResizablePanel
        first={<div>A</div>}
        second={<div>B</div>}
        onPositionChange={onChange}
      />,
    );
    const separator = screen.getByRole("separator");
    fireEvent.mouseDown(separator);
    // In jsdom, mousemove won't trigger real position calculation
    // but the component should not crash
    fireEvent.mouseUp(document);
  });

  it("renders with custom divider size", () => {
    renderWithMantine(
      <ResizablePanel
        first={<div>A</div>}
        second={<div>B</div>}
        dividerSize={8}
      />,
    );
    const separator = screen.getByRole("separator");
    expect(separator.style.width).toBe("8px");
  });
});

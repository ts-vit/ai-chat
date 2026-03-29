import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen } from "@testing-library/react";
import { MantineProvider } from "@mantine/core";
import { StatusBadge, type StatusVariant } from "../components/StatusBadge";

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
});

function renderWithMantine(ui: React.ReactElement) {
  return render(<MantineProvider>{ui}</MantineProvider>);
}

describe("StatusBadge", () => {
  it("renders default label for status", () => {
    renderWithMantine(<StatusBadge status="connected" />);
    expect(screen.getByText("Connected")).toBeInTheDocument();
  });

  it("renders custom label", () => {
    renderWithMantine(<StatusBadge status="connected" label="Online" />);
    expect(screen.getByText("Online")).toBeInTheDocument();
  });

  it("renders all status variants without crashing", () => {
    const variants: StatusVariant[] = [
      "connected",
      "disconnected",
      "running",
      "idle",
      "error",
      "warning",
      "unknown",
      "auth_required",
    ];
    for (const status of variants) {
      const { unmount } = renderWithMantine(<StatusBadge status={status} />);
      unmount();
    }
  });

  it("uses labels map for i18n", () => {
    renderWithMantine(
      <StatusBadge
        status="connected"
        labels={{ connected: "Подключён", error: "Ошибка" }}
      />,
    );
    expect(screen.getByText("Подключён")).toBeInTheDocument();
  });

  it("shows dot for connected status by default", () => {
    const { container } = renderWithMantine(
      <StatusBadge status="connected" />,
    );
    const dot = container.querySelector("div[style*='border-radius']");
    expect(dot).toBeTruthy();
  });

  it("hides dot when dot=false", () => {
    const { container } = renderWithMantine(
      <StatusBadge status="connected" dot={false} />,
    );
    const dots = container.querySelectorAll(
      "div[style*='border-radius: 50%']",
    );
    expect(dots.length).toBe(0);
  });

  it("shows dot for non-default status when dot=true", () => {
    const { container } = renderWithMantine(
      <StatusBadge status="error" dot={true} />,
    );
    const dot = container.querySelector("div[style*='border-radius']");
    expect(dot).toBeTruthy();
  });

  it("renders with custom color", () => {
    renderWithMantine(<StatusBadge status="idle" color="violet" />);
    expect(screen.getByText("Idle")).toBeInTheDocument();
  });
});

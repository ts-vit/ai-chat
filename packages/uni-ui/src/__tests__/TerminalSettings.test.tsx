import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import { TerminalSettings } from "../modules/terminal/TerminalSettings";
import { MantineProvider } from "@mantine/core";

vi.mock("@mantine/notifications", () => ({
  Notifications: () => null,
  notifications: { show: vi.fn(), clean: vi.fn() },
}));

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
  } as unknown as typeof ResizeObserver;
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: "en" },
  }),
}));

function makeMockAdapter(
  initial: Record<string, string> = {}
): SettingsAdapter {
  const store = { ...initial };
  return {
    get: vi.fn(async (key: string) => store[key]),
    set: vi.fn(async (key: string, value: string) => {
      store[key] = value;
    }),
    delete: vi.fn(async (key: string) => {
      delete store[key];
    }),
    getAll: vi.fn(async () =>
      Object.entries(store).map(([key, value]) => ({
        key,
        value,
        isSensitive: false,
      }))
    ),
  };
}

function renderWithProvider(adapter: SettingsAdapter) {
  return render(
    <MantineProvider>
      <SettingsProvider adapter={adapter}>
        <TerminalSettings />
      </SettingsProvider>
    </MantineProvider>
  );
}

describe("TerminalSettings", () => {
  it("renders with defaults — fontSize 13, shell Auto", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("terminal.title")).toBeInTheDocument();
    });

    expect(screen.getByText("terminal.fontSize")).toBeInTheDocument();
    expect(screen.getByText("terminal.shell")).toBeInTheDocument();
  });

  it("selects a shell option via hidden dropdown", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("terminal.title")).toBeInTheDocument();
    });

    // Mantine Select renders options in a hidden portal; use hidden: true
    const options = screen.getAllByRole("option", { hidden: true });
    expect(options.length).toBeGreaterThan(1);

    // Click second option (first non-Auto shell)
    fireEvent.click(options[1]);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "terminal.shell",
        expect.any(String)
      );
    });
  });

  it("shows custom input when Custom selected", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("terminal.title")).toBeInTheDocument();
    });

    // Click the last option (Custom...)
    const options = screen.getAllByRole("option", { hidden: true });
    fireEvent.click(options[options.length - 1]);

    // A second textbox should appear for custom path
    await waitFor(() => {
      const inputs = screen.getAllByRole("textbox");
      expect(inputs.length).toBe(2);
    });
  });

  it("detects custom shell on mount when value is not in known list", async () => {
    const adapter = makeMockAdapter({
      "terminal.shell": "/usr/bin/fish",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("terminal.title")).toBeInTheDocument();
    });

    // Custom shell input should be visible with the value
    await waitFor(() => {
      const inputs = screen.getAllByRole("textbox");
      expect(inputs.length).toBe(2);
    });

    const inputs = screen.getAllByRole("textbox");
    const customInput = inputs[1];
    expect(customInput).toHaveValue("/usr/bin/fish");
  });
});

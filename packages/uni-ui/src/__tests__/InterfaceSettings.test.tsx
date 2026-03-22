import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import { InterfaceSettings } from "../modules/interface/InterfaceSettings";
import { MantineProvider } from "@mantine/core";

vi.mock("@mantine/notifications", () => ({
  Notifications: () => null,
  notifications: { show: vi.fn(), clean: vi.fn() },
}));

const mockSetColorScheme = vi.fn();

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

vi.mock("@mantine/core", async () => {
  const actual = await vi.importActual("@mantine/core");
  return {
    ...actual,
    useMantineColorScheme: () => ({
      colorScheme: "dark",
      setColorScheme: mockSetColorScheme,
    }),
  };
});

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
        <InterfaceSettings />
      </SettingsProvider>
    </MantineProvider>
  );
}

describe("InterfaceSettings", () => {
  it("renders with defaults when all settings are null", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.interface.theme")
      ).toBeInTheDocument();
    });

    expect(
      screen.getByText("settings.interface.fontSize")
    ).toBeInTheDocument();
    expect(
      screen.getByText("settings.interface.language")
    ).toBeInTheDocument();
    expect(
      screen.getByText("settings.interface.sendMethod")
    ).toBeInTheDocument();
  });

  it("calls adapter.set on language change", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.interface.language")
      ).toBeInTheDocument();
    });

    // Find the select input and change it
    const select = screen.getByRole("textbox");
    fireEvent.click(select);

    // Find the "ru" option
    const ruOption = await screen.findByText("settings.interface.languageRu");
    fireEvent.click(ruOption);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("ui.language", "ru");
    });
  });

  it("calls adapter.set on density change", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.interface.messageDensity")
      ).toBeInTheDocument();
    });

    const compactLabel = screen.getByText(
      "settings.interface.densityCompact"
    );
    fireEvent.click(compactLabel);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "ui.message_density",
        "compact"
      );
    });
  });

  it("calls adapter.set on status bar toggle", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.interface.showStatusBar")
      ).toBeInTheDocument();
    });

    // Mantine Switch renders an input[type=checkbox] inside
    const switchLabel = screen.getByText("settings.interface.showStatusBar");
    const switchEl = switchLabel.closest("label")?.querySelector("input");
    expect(switchEl).toBeTruthy();
    fireEvent.click(switchEl!);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("ui.show_status_bar", "false");
    });
  });

  it("calls adapter.set on metrics checkbox change", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.interface.statusBarMetrics")
      ).toBeInTheDocument();
    });

    // Uncheck the "balance" metric
    const balanceCheckbox = screen.getByRole("checkbox", {
      name: "settings.interface.metricBalance",
    });
    fireEvent.click(balanceCheckbox);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "ui.status_bar_metrics",
        expect.any(String)
      );
      // Should contain context, tokens, cost but not balance
      const call = (adapter.set as ReturnType<typeof vi.fn>).mock.calls.find(
        (c: string[]) => c[0] === "ui.status_bar_metrics"
      );
      expect(call).toBeDefined();
      const parsed = JSON.parse(call![1]);
      expect(parsed).not.toContain("balance");
      expect(parsed).toContain("context");
    });
  });

  it("theme uses Mantine setColorScheme, not adapter.set", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.interface.theme")
      ).toBeInTheDocument();
    });

    const lightLabel = screen.getByText("settings.interface.themeLight");
    fireEvent.click(lightLabel);

    expect(mockSetColorScheme).toHaveBeenCalledWith("light");
    // adapter.set should NOT be called for theme
    const setCalls = (adapter.set as ReturnType<typeof vi.fn>).mock.calls;
    const themeCalls = setCalls.filter(
      (c: string[]) =>
        c[0].includes("theme") || c[0].includes("color_scheme")
    );
    expect(themeCalls).toHaveLength(0);
  });
});

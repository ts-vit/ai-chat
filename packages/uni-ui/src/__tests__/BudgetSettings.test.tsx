import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import { BudgetSettings } from "../modules/budget/BudgetSettings";
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
        <BudgetSettings />
      </SettingsProvider>
    </MantineProvider>
  );
}

describe("BudgetSettings", () => {
  it("renders with defaults — both switches off, no number inputs", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("budget.perPlan")).toBeInTheDocument();
    });

    expect(screen.getByText("budget.global")).toBeInTheDocument();

    // No limit fields visible when disabled
    expect(screen.queryByText("budget.limit")).not.toBeInTheDocument();
  });

  it("enables per-plan budget on switch toggle", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("budget.perPlan")).toBeInTheDocument();
    });

    const perPlanSwitch = screen.getByText("budget.perPlan")
      .closest("label")!
      .querySelector("input")!;
    await act(async () => {
      fireEvent.click(perPlanSwitch);
    });

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "budget.plan.enabled",
        "true"
      );
    });
  });

  it("enables global budget on switch toggle", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("budget.global")).toBeInTheDocument();
    });

    const globalSwitch = screen.getByText("budget.global")
      .closest("label")!
      .querySelector("input")!;
    await act(async () => {
      fireEvent.click(globalSwitch);
    });

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "budget.global.enabled",
        "true"
      );
    });
  });

  it("shows period select and limit when global enabled", async () => {
    const adapter = makeMockAdapter({
      "budget.global.enabled": "true",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("budget.period")).toBeInTheDocument();
    });
  });

  it("shows limit input when per-plan enabled", async () => {
    const adapter = makeMockAdapter({
      "budget.plan.enabled": "true",
      "budget.plan.limit": "5",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("budget.limit")).toBeInTheDocument();
    });
  });
});

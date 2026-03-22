import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import { GenerationSettings } from "../modules/generation/GenerationSettings";
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
    i18n: { language: "en" },
  }),
}));

function makeMockAdapter(initial: Record<string, string> = {}): SettingsAdapter {
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
        <GenerationSettings />
      </SettingsProvider>
    </MantineProvider>
  );
}

describe("GenerationSettings", () => {
  it("renders with defaults when all settings are null", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    // Wait for loading to finish
    await waitFor(() => {
      expect(screen.getByText("settings.generation.title")).toBeInTheDocument();
    });

    // Temperature slider should exist
    expect(screen.getByText("settings.generation.temperature")).toBeInTheDocument();
    expect(screen.getByText("settings.generation.maxTokens")).toBeInTheDocument();
  });

  it("shows optional parameters as unchecked when keys absent", async () => {
    const adapter = makeMockAdapter({
      "llm.temperature": "0.7",
      "llm.max_tokens": "4096",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.generation.title")).toBeInTheDocument();
    });

    // All 4 optional checkboxes should be unchecked
    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes).toHaveLength(4);
    checkboxes.forEach((cb) => {
      expect(cb).not.toBeChecked();
    });
  });

  it("shows optional parameter as checked when key exists", async () => {
    const adapter = makeMockAdapter({
      "llm.temperature": "0.5",
      "llm.max_tokens": "2048",
      "llm.top_p": "0.85",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.generation.title")).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByRole("checkbox");
    // topP checkbox (first) should be checked
    expect(checkboxes[0]).toBeChecked();
    // Others unchecked
    expect(checkboxes[1]).not.toBeChecked();
    expect(checkboxes[2]).not.toBeChecked();
    expect(checkboxes[3]).not.toBeChecked();
  });

  it("enables optional parameter on checkbox click", async () => {
    const adapter = makeMockAdapter({
      "llm.temperature": "0.7",
      "llm.max_tokens": "4096",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.generation.title")).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByRole("checkbox");
    // Click topP checkbox (first one)
    fireEvent.click(checkboxes[0]);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("llm.top_p", "0.9");
    });
  });

  it("disables optional parameter on checkbox uncheck", async () => {
    const adapter = makeMockAdapter({
      "llm.temperature": "0.7",
      "llm.max_tokens": "4096",
      "llm.top_p": "0.85",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.generation.title")).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByRole("checkbox");
    // topP checkbox should be checked, click to uncheck
    expect(checkboxes[0]).toBeChecked();
    fireEvent.click(checkboxes[0]);

    await waitFor(() => {
      expect(adapter.delete).toHaveBeenCalledWith("llm.top_p");
    });
  });

  it("saves maxTokens on blur", async () => {
    const adapter = makeMockAdapter({
      "llm.temperature": "0.7",
      "llm.max_tokens": "4096",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.generation.title")).toBeInTheDocument();
    });

    const maxTokensInput = screen.getByRole("textbox");
    fireEvent.change(maxTokensInput, { target: { value: "8192" } });
    fireEvent.blur(maxTokensInput);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("llm.max_tokens", expect.any(String));
    });
  });
});

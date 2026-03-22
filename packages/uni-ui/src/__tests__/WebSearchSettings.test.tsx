import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import { WebSearchSettings } from "../modules/web-search/WebSearchSettings";
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
        <WebSearchSettings />
      </SettingsProvider>
    </MantineProvider>
  );
}

describe("WebSearchSettings", () => {
  it("renders with defaults — no API key fields visible", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.webSearch.title")
      ).toBeInTheDocument();
    });

    expect(
      screen.getByText("settings.webSearch.provider")
    ).toBeInTheDocument();

    // No API key fields when provider is empty
    expect(
      screen.queryByText("settings.webSearch.tavilyApiKey")
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText("settings.webSearch.braveApiKey")
    ).not.toBeInTheDocument();
  });

  it("shows Tavily API key field when Tavily is selected", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.webSearch.title")
      ).toBeInTheDocument();
    });

    // Open provider select and choose Tavily
    const select = screen.getByRole("textbox");
    fireEvent.click(select);

    const tavilyOption = await screen.findByText("Tavily");
    fireEvent.click(tavilyOption);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("search.provider", "tavily");
    });

    expect(
      screen.getByText("settings.webSearch.tavilyApiKey")
    ).toBeInTheDocument();
    expect(
      screen.getByText("settings.webSearch.tavilyHint")
    ).toBeInTheDocument();
  });

  it("shows Brave API key field when Brave is selected", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.webSearch.title")
      ).toBeInTheDocument();
    });

    const select = screen.getByRole("textbox");
    fireEvent.click(select);

    const braveOption = await screen.findByText("Brave Search");
    fireEvent.click(braveOption);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("search.provider", "brave");
    });

    expect(
      screen.getByText("settings.webSearch.braveApiKey")
    ).toBeInTheDocument();
    expect(
      screen.getByText("settings.webSearch.braveHint")
    ).toBeInTheDocument();
  });

  it("shows UNI description when UNI is selected", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.webSearch.title")
      ).toBeInTheDocument();
    });

    const select = screen.getByRole("textbox");
    fireEvent.click(select);

    const uniOption = await screen.findByText("UNI Search");
    fireEvent.click(uniOption);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("search.provider", "uni");
    });

    expect(
      screen.getByText("settings.webSearch.uniDescription")
    ).toBeInTheDocument();
  });

  it("saves Tavily API key onBlur", async () => {
    const adapter = makeMockAdapter({
      "search.provider": "tavily",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.webSearch.tavilyApiKey")
      ).toBeInTheDocument();
    });

    const input = screen.getByLabelText("settings.webSearch.tavilyApiKey");
    fireEvent.change(input, { target: { value: "tvly-test-key" } });
    fireEvent.blur(input);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "search.tavily.api_key",
        "tvly-test-key"
      );
    });
  });
});

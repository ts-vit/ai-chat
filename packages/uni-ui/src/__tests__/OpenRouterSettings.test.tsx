import { describe, it, expect, vi, beforeEach, afterEach, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor, renderHook, act } from "@testing-library/react";
import type { ReactNode } from "react";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import { useOpenRouterModels } from "../modules/openrouter/useOpenRouterModels";
import { ModelCatalog } from "../modules/openrouter/ModelCatalog";
import { OpenRouterSettings } from "../modules/openrouter/OpenRouterSettings";
import { MantineProvider } from "@mantine/core";

vi.mock("@mantine/notifications", () => ({
  Notifications: () => null,
  notifications: { show: vi.fn(), clean: vi.fn() },
}));

// Mantine requires window.matchMedia and ResizeObserver in jsdom
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

// i18n mock — returns key as-is
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts?.count !== undefined) return `${key}:${opts.count}`;
      return key;
    },
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

const MOCK_MODELS = [
  {
    id: "anthropic/claude-3-opus",
    name: "Claude 3 Opus",
    pricing: { prompt: "0.000015", completion: "0.000075" },
    context_length: 200000,
    description: "Most capable model",
  },
  {
    id: "openai/gpt-4o",
    name: "GPT-4o",
    pricing: { prompt: "0.000005", completion: "0.000015" },
    context_length: 128000,
  },
  {
    id: "meta-llama/llama-3-8b",
    name: "Llama 3 8B",
    pricing: { prompt: "0", completion: "0" },
    context_length: 8192,
  },
];

function TestWrapper({ adapter, children }: { adapter: SettingsAdapter; children: ReactNode }) {
  return (
    <MantineProvider>
      <SettingsProvider adapter={adapter}>{children}</SettingsProvider>
    </MantineProvider>
  );
}

// ── useOpenRouterModels ──

describe("useOpenRouterModels", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("loads models on mount", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ data: MOCK_MODELS }),
    });

    const { result } = renderHook(() => useOpenRouterModels());

    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.models).toHaveLength(3);
    expect(result.current.error).toBeNull();
  });

  it("returns error on fetch failure", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({}),
    });

    const { result } = renderHook(() => useOpenRouterModels());

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("HTTP 500");
    expect(result.current.models).toHaveLength(0);
  });

  it("reload(true) refetches", async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn().mockImplementation(async () => {
      callCount++;
      return {
        ok: true,
        json: async () => ({ data: MOCK_MODELS }),
      };
    });

    const { result } = renderHook(() => useOpenRouterModels());

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(callCount).toBe(1);

    act(() => {
      result.current.reload(true);
    });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(callCount).toBe(2);
  });
});

// ── ModelCatalog ──

describe("ModelCatalog", () => {
  const defaultProps = {
    models: MOCK_MODELS,
    loading: false,
    error: null,
    onReload: vi.fn(),
    selectedIds: [] as string[],
    onToggleModel: vi.fn(),
  };

  it("renders model rows", () => {
    render(
      <MantineProvider>
        <ModelCatalog {...defaultProps} />
      </MantineProvider>
    );

    expect(screen.getByText("Claude 3 Opus")).toBeInTheDocument();
    expect(screen.getByText("GPT-4o")).toBeInTheDocument();
    expect(screen.getByText("Llama 3 8B")).toBeInTheDocument();
  });

  it("filters free models", async () => {
    render(
      <MantineProvider>
        <ModelCatalog {...defaultProps} />
      </MantineProvider>
    );

    // Click "Free" segment
    const freeBtn = screen.getByText("settings.openrouter.free");
    fireEvent.click(freeBtn);

    expect(screen.getByText("Llama 3 8B")).toBeInTheDocument();
    expect(screen.queryByText("Claude 3 Opus")).not.toBeInTheDocument();
    expect(screen.queryByText("GPT-4o")).not.toBeInTheDocument();
  });

  it("filters selected models", () => {
    render(
      <MantineProvider>
        <ModelCatalog
          {...defaultProps}
          selectedIds={["openai/gpt-4o"]}
        />
      </MantineProvider>
    );

    const selectedBtn = screen.getByText("settings.openrouter.selected");
    fireEvent.click(selectedBtn);

    expect(screen.getByText("GPT-4o")).toBeInTheDocument();
    expect(screen.queryByText("Claude 3 Opus")).not.toBeInTheDocument();
  });

  it("search filters by name/id", () => {
    render(
      <MantineProvider>
        <ModelCatalog {...defaultProps} />
      </MantineProvider>
    );

    const searchInput = screen.getByPlaceholderText("settings.openrouter.searchPlaceholder");
    fireEvent.change(searchInput, { target: { value: "claude" } });

    expect(screen.getByText("Claude 3 Opus")).toBeInTheDocument();
    expect(screen.queryByText("GPT-4o")).not.toBeInTheDocument();
  });

  it("calls onToggleModel when checkbox clicked", () => {
    const onToggle = vi.fn();
    render(
      <MantineProvider>
        <ModelCatalog {...defaultProps} onToggleModel={onToggle} />
      </MantineProvider>
    );

    const checkboxes = screen.getAllByRole("checkbox");
    fireEvent.click(checkboxes[0]);
    expect(onToggle).toHaveBeenCalledWith("anthropic/claude-3-opus");
  });

  it("no limit on selections — all checkboxes enabled", () => {
    render(
      <MantineProvider>
        <ModelCatalog
          {...defaultProps}
          selectedIds={[
            "anthropic/claude-3-opus",
            "openai/gpt-4o",
            "meta-llama/llama-3-8b",
          ]}
        />
      </MantineProvider>
    );

    const checkboxes = screen.getAllByRole("checkbox");
    checkboxes.forEach((cb) => {
      expect(cb).not.toBeDisabled();
    });
  });

  it("shows loading state", () => {
    const { container } = render(
      <MantineProvider>
        <ModelCatalog {...defaultProps} loading={true} models={[]} />
      </MantineProvider>
    );

    // Mantine Loader renders as a span with mantine-Loader-root class
    expect(container.querySelector(".mantine-Loader-root")).toBeInTheDocument();
  });

  it("shows error with retry", () => {
    const onReload = vi.fn();
    render(
      <MantineProvider>
        <ModelCatalog
          {...defaultProps}
          error="Network error"
          onReload={onReload}
        />
      </MantineProvider>
    );

    expect(screen.getByText("Network error")).toBeInTheDocument();
    fireEvent.click(screen.getByText("settings.openrouter.retry"));
    expect(onReload).toHaveBeenCalled();
  });
});

// ── OpenRouterSettings ──

describe("OpenRouterSettings", () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ data: MOCK_MODELS }),
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("renders without errors", async () => {
    const adapter = makeMockAdapter({
      "llm.openrouter.api_key": "sk-test",
      "llm.openrouter.models": "[]",
    });

    render(
      <TestWrapper adapter={adapter}>
        <OpenRouterSettings />
      </TestWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText("settings.openrouter.apiKey")).toBeInTheDocument();
      expect(screen.getByText("settings.openrouter.managementKey")).toBeInTheDocument();
    });
  });

  it("toggles model and calls adapter.set with updated JSON", async () => {
    const adapter = makeMockAdapter({
      "llm.openrouter.api_key": "",
      "llm.openrouter.mgmt_key": "",
      "llm.openrouter.models": "[]",
    });

    render(
      <TestWrapper adapter={adapter}>
        <OpenRouterSettings />
      </TestWrapper>
    );

    // Wait for models to load
    await waitFor(() => {
      expect(screen.getByText("Claude 3 Opus")).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByRole("checkbox");
    fireEvent.click(checkboxes[0]);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "llm.openrouter.models",
        JSON.stringify(["anthropic/claude-3-opus"])
      );
    });
  });
});

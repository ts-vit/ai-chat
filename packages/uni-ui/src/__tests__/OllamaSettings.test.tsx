import { describe, it, expect, vi, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import { OllamaSettings } from "../modules/ollama/OllamaSettings";
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
    t: (key: string, params?: Record<string, unknown>) => {
      if (params) {
        let result = key;
        for (const [k, v] of Object.entries(params)) {
          result = result.replace(`{{${k}}}`, String(v));
        }
        return result;
      }
      return key;
    },
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
        <OllamaSettings />
      </SettingsProvider>
    </MantineProvider>
  );
}

// Mock fetch for Ollama API
const mockFetch = vi.fn();
beforeEach(() => {
  mockFetch.mockReset();
  globalThis.fetch = mockFetch;
});

describe("OllamaSettings", () => {
  it("renders URL input and starts checking status", async () => {
    mockFetch.mockRejectedValue(new Error("network"));
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.ollama.url")).toBeInTheDocument();
    });

    expect(screen.getByPlaceholderText("ollama.serverUrlPlaceholder")).toBeInTheDocument();
  });

  it("shows 'Running' when Ollama is available", async () => {
    mockFetch.mockImplementation(async (url: string) => {
      if (url === "http://localhost:11434") {
        return { ok: true };
      }
      if (url === "http://localhost:11434/api/tags") {
        return {
          ok: true,
          json: async () => ({ models: [] }),
        };
      }
      return { ok: false };
    });

    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("ollama.running")).toBeInTheDocument();
    });
  });

  it("shows 'Not found' and Install button when unavailable", async () => {
    mockFetch.mockRejectedValue(new Error("network"));
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("ollama.notFound")).toBeInTheDocument();
    });

    expect(screen.getByText("ollama.installOllama")).toBeInTheDocument();
  });

  it("renders model list with checkboxes when models exist", async () => {
    mockFetch.mockImplementation(async (url: string) => {
      if (url === "http://localhost:11434") {
        return { ok: true };
      }
      if (url === "http://localhost:11434/api/tags") {
        return {
          ok: true,
          json: async () => ({
            models: [
              { name: "qwen2.5:3b", size: 2000000000 },
              { name: "llama3.2:3b", size: 2100000000 },
            ],
          }),
        };
      }
      return { ok: false };
    });

    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("qwen2.5:3b")).toBeInTheDocument();
    });

    expect(screen.getByText("llama3.2:3b")).toBeInTheDocument();
    expect(screen.getByText("2.0 GB")).toBeInTheDocument();
    expect(screen.getByText("2.1 GB")).toBeInTheDocument();
  });

  it("toggles model selection and saves to settings", async () => {
    mockFetch.mockImplementation(async (url: string) => {
      if (url === "http://localhost:11434") {
        return { ok: true };
      }
      if (url === "http://localhost:11434/api/tags") {
        return {
          ok: true,
          json: async () => ({
            models: [{ name: "qwen2.5:3b", size: 2000000000 }],
          }),
        };
      }
      return { ok: false };
    });

    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("qwen2.5:3b")).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByRole("checkbox");
    fireEvent.click(checkboxes[0]);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "llm.ollama.models",
        JSON.stringify(["qwen2.5:3b"])
      );
    });
  });

  it("allows selecting unlimited models (no MAX limit)", async () => {
    mockFetch.mockImplementation(async (url: string) => {
      if (url === "http://localhost:11434") {
        return { ok: true };
      }
      if (url === "http://localhost:11434/api/tags") {
        return {
          ok: true,
          json: async () => ({
            models: Array.from({ length: 8 }, (_, i) => ({
              name: `model${i}:latest`,
              size: 1000000000,
            })),
          }),
        };
      }
      return { ok: false };
    });

    const adapter = makeMockAdapter({
      "llm.ollama.models": JSON.stringify(
        Array.from({ length: 6 }, (_, i) => `model${i}:latest`)
      ),
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("model0:latest")).toBeInTheDocument();
    });

    // All checkboxes should be enabled (no disabled ones)
    const checkboxes = screen.getAllByRole("checkbox");
    checkboxes.forEach((cb) => {
      expect(cb).not.toBeDisabled();
    });
  });

  it("saves URL on blur", async () => {
    mockFetch.mockRejectedValue(new Error("network"));
    const adapter = makeMockAdapter({
      "llm.ollama.url": "http://localhost:11434/v1",
    });
    renderWithProvider(adapter);

    // Wait for settings to load and status check to settle
    await waitFor(() => {
      expect(screen.getByText("ollama.notFound")).toBeInTheDocument();
    });

    const urlInput = screen.getByPlaceholderText("ollama.serverUrlPlaceholder");
    fireEvent.change(urlInput, {
      target: { value: "http://custom:11434/v1" },
    });
    fireEvent.blur(urlInput);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith(
        "llm.ollama.url",
        "http://custom:11434/v1"
      );
    });
  });
});

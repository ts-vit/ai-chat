import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { useSettings } from "../settings/useSettings";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";

// Mock adapter factory
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

function makeWrapper(adapter: SettingsAdapter) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <SettingsProvider adapter={adapter}>{children}</SettingsProvider>
    );
  };
}

describe("useSettings", () => {
  let adapter: SettingsAdapter;

  beforeEach(() => {
    adapter = makeMockAdapter({ "llm.openrouter.api_key": "sk-initial" });
  });

  it("loads value on mount", async () => {
    const { result } = renderHook(
      () => useSettings("llm.openrouter.api_key"),
      { wrapper: makeWrapper(adapter) }
    );

    expect(result.current.loading).toBe(true);
    expect(result.current.value).toBeUndefined();

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.value).toBe("sk-initial");
  });

  it("returns null for missing key", async () => {
    const { result } = renderHook(
      () => useSettings("nonexistent.key"),
      { wrapper: makeWrapper(adapter) }
    );

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.value).toBeNull();
  });

  it("set updates value after adapter call", async () => {
    const { result } = renderHook(
      () => useSettings("llm.openrouter.api_key"),
      { wrapper: makeWrapper(adapter) }
    );

    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.set("sk-new");
    });

    expect(result.current.value).toBe("sk-new");
    expect(adapter.set).toHaveBeenCalledWith(
      "llm.openrouter.api_key",
      "sk-new"
    );
  });

  it("delete sets value to null", async () => {
    const { result } = renderHook(
      () => useSettings("llm.openrouter.api_key"),
      { wrapper: makeWrapper(adapter) }
    );

    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.delete();
    });

    expect(result.current.value).toBeNull();
    expect(adapter.delete).toHaveBeenCalledWith("llm.openrouter.api_key");
  });

  it("refresh reloads value from adapter", async () => {
    const { result } = renderHook(
      () => useSettings("llm.openrouter.api_key"),
      { wrapper: makeWrapper(adapter) }
    );

    await waitFor(() => expect(result.current.loading).toBe(false));

    // Simulate backend change
    vi.mocked(adapter.get).mockResolvedValueOnce("sk-refreshed");

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.value).toBe("sk-refreshed");
  });

  it("throws when used outside SettingsProvider", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() =>
      renderHook(() => useSettings("any.key"))
    ).toThrow("useSettingsAdapter");
    spy.mockRestore();
  });
});

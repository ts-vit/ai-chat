import { describe, it, expect, vi, beforeEach } from "vitest";
import { TauriSettingsAdapter } from "../settings/TauriSettingsAdapter";

describe("TauriSettingsAdapter", () => {
  const mockInvoke = vi.fn();
  let adapter: TauriSettingsAdapter;

  beforeEach(() => {
    mockInvoke.mockClear();
    adapter = new TauriSettingsAdapter(mockInvoke);
  });

  it("get — calls get_setting with correct key", async () => {
    mockInvoke.mockResolvedValue("sk-123");
    const result = await adapter.get("llm.openrouter.api_key");
    expect(mockInvoke).toHaveBeenCalledWith("get_setting", {
      key: "llm.openrouter.api_key",
    });
    expect(result).toBe("sk-123");
  });

  it("get — returns undefined when backend returns null", async () => {
    mockInvoke.mockResolvedValue(null);
    const result = await adapter.get("llm.openrouter.api_key");
    expect(result).toBeUndefined();
  });

  it("set — calls set_setting with key and value", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await adapter.set("llm.openrouter.api_key", "sk-abc");
    expect(mockInvoke).toHaveBeenCalledWith("set_setting", {
      key: "llm.openrouter.api_key",
      value: "sk-abc",
    });
  });

  it("delete — calls delete_setting with key", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await adapter.delete("llm.openrouter.api_key");
    expect(mockInvoke).toHaveBeenCalledWith("delete_setting", {
      key: "llm.openrouter.api_key",
    });
  });

  it("getAll — calls get_all_settings with empty prefix by default", async () => {
    mockInvoke.mockResolvedValue([]);
    await adapter.getAll();
    expect(mockInvoke).toHaveBeenCalledWith("get_all_settings", {
      prefix: "",
    });
  });

  it("getAll — passes prefix to get_all_settings", async () => {
    mockInvoke.mockResolvedValue([]);
    await adapter.getAll("llm.");
    expect(mockInvoke).toHaveBeenCalledWith("get_all_settings", {
      prefix: "llm.",
    });
  });

  it("getAll — maps snake_case is_sensitive to camelCase isSensitive", async () => {
    mockInvoke.mockResolvedValue([
      { key: "llm.openrouter.api_key", value: "sk-***", is_sensitive: true },
      { key: "llm.openrouter.model", value: "gpt-4", is_sensitive: false },
    ]);
    const result = await adapter.getAll();
    expect(result).toEqual([
      { key: "llm.openrouter.api_key", value: "sk-***", isSensitive: true },
      { key: "llm.openrouter.model", value: "gpt-4", isSensitive: false },
    ]);
  });
});

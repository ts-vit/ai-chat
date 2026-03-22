import { useState, useEffect, useCallback } from "react";
import { useSettingsAdapter } from "./SettingsContext";

export interface UseSettingsResult {
  /** Current value. undefined = not loaded yet, null = key not set. */
  value: string | null | undefined;
  /** True while loading. */
  loading: boolean;
  /** Set a new value. */
  set(value: string): Promise<void>;
  /** Delete the key. */
  delete(): Promise<void>;
  /** Reload from backend. */
  refresh(): Promise<void>;
}

/**
 * Read and write a single setting by key.
 *
 * Example:
 *   const { value, set } = useSettings("llm.openrouter.api_key");
 */
export function useSettings(key: string): UseSettingsResult {
  const adapter = useSettingsAdapter();
  const [value, setValue] = useState<string | null | undefined>(undefined);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const v = await adapter.get(key);
      setValue(v ?? null);
    } finally {
      setLoading(false);
    }
  }, [adapter, key]);

  useEffect(() => {
    load();
  }, [load]);

  const set = useCallback(
    async (newValue: string) => {
      await adapter.set(key, newValue);
      setValue(newValue);
    },
    [adapter, key]
  );

  const del = useCallback(async () => {
    await adapter.delete(key);
    setValue(null);
  }, [adapter, key]);

  return {
    value,
    loading,
    set,
    delete: del,
    refresh: load,
  };
}

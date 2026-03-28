import type { SettingsAdapter, SettingEntry } from "./types";

/**
 * Invoke function type — matches @tauri-apps/api/core invoke signature.
 * Passed as constructor parameter to keep @uni-fw/ui free of Tauri dependency.
 */
type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * Standard settings adapter for Tauri applications.
 * Uses uni-settings commands: get_setting, set_setting, delete_setting, get_all_settings.
 *
 * Usage:
 *   import { invoke } from "@tauri-apps/api/core";
 *   const adapter = new TauriSettingsAdapter(invoke);
 *   <UniProvider settingsAdapter={adapter}>
 */
export class TauriSettingsAdapter implements SettingsAdapter {
  constructor(private readonly invoke: InvokeFn) {}

  async get(key: string): Promise<string | undefined> {
    const result = await this.invoke<string | null>("get_setting", { key });
    return result ?? undefined;
  }

  async set(key: string, value: string): Promise<void> {
    await this.invoke<void>("set_setting", { key, value });
  }

  async delete(key: string): Promise<void> {
    await this.invoke<void>("delete_setting", { key });
  }

  async getAll(prefix = ""): Promise<SettingEntry[]> {
    const result = await this.invoke<
      Array<{
        key: string;
        value: string;
        is_sensitive: boolean;
      }>
    >("get_all_settings", { prefix });
    return result.map((s) => ({
      key: s.key,
      value: s.value,
      isSensitive: s.is_sensitive,
    }));
  }
}

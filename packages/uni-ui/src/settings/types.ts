/**
 * Single setting entry returned by getAll().
 * Sensitive values (api_key, password, token, secret) are masked by backend.
 */
export interface SettingEntry {
  key: string;
  value: string;
  isSensitive: boolean;
}

/**
 * Settings adapter interface.
 * Implement this to connect @uni-fw/ui modules to any settings backend.
 * Standard implementation: TauriSettingsAdapter.
 */
export interface SettingsAdapter {
  /** Get a single value. Returns undefined if key not set. */
  get(key: string): Promise<string | undefined>;
  /** Set a value. */
  set(key: string, value: string): Promise<void>;
  /** Delete a key. */
  delete(key: string): Promise<void>;
  /** Get all settings, optionally filtered by prefix. */
  getAll(prefix?: string): Promise<SettingEntry[]>;
}

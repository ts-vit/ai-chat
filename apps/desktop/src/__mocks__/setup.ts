import { afterEach } from 'vitest'
import { clearInvokeMocks } from './@tauri-apps/api/core'

if (typeof globalThis.localStorage === "undefined") {
  (globalThis as Record<string, unknown>).localStorage = {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    length: 0,
    key: () => null,
  };
}

afterEach(() => {
  clearInvokeMocks();
});

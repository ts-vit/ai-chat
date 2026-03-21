import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/__tests__/*.test.ts"],
    environment: "node",
    setupFiles: ["./src/__mocks__/setup.ts"],
    alias: {
      "@tauri-apps/api/core": "./src/__mocks__/@tauri-apps/api/core.ts",
      "@tauri-apps/api/event": "./src/__mocks__/@tauri-apps/api/event.ts",
      "@tauri-apps/api/path": "./src/__mocks__/@tauri-apps/api/path.ts",
      "@tauri-apps/plugin-clipboard-manager": "./src/__mocks__/@tauri-apps/plugin-clipboard-manager.ts",
      "@tauri-apps/plugin-dialog": "./src/__mocks__/@tauri-apps/plugin-dialog.ts",
      "@tauri-apps/plugin-opener": "./src/__mocks__/@tauri-apps/plugin-opener.ts",
      "@tauri-apps/plugin-shell": "./src/__mocks__/@tauri-apps/plugin-shell.ts",
      "@tauri-apps/plugin-fs": "./src/__mocks__/@tauri-apps/plugin-fs.ts",
    },
  },
});

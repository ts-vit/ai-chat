// Theme
export { brandOrange, uniCssResolver, uniTheme } from "./theme";

// Components
export { UniProvider, type UniProviderProps } from "./components";
export { MarkdownRenderer, type MarkdownRendererProps } from "./components";
export { ConfirmModal, type ConfirmModalProps } from "./components";

// Settings
export type { SettingsAdapter, SettingEntry } from "./settings";
export { TauriSettingsAdapter } from "./settings";
export { SettingsProvider, useSettingsAdapter } from "./settings";
export { useSettings, type UseSettingsResult } from "./settings";

// Modules
export * from "./modules";

// Re-export Mantine for convenience — apps import from @uni/ui instead of @mantine/core directly
export * from "@mantine/core";
export * from "@mantine/hooks";
export { Notifications, notifications } from "@mantine/notifications";

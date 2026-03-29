// Theme
export { brandOrange, uniCssResolver, uniTheme } from "./theme";

// Components
export { UniProvider, type UniProviderProps } from "./components";
export { MarkdownRenderer, type MarkdownRendererProps } from "./components";
export { ConfirmModal, type ConfirmModalProps } from "./components";
export { SessionTabs } from "./components";
export type { SessionTabsProps, SessionTab, TabStatus } from "./components";
export { ResizablePanel } from "./components";
export type { ResizablePanelProps, SplitDirection } from "./components";
export { EmptyState } from "./components";
export type { EmptyStateProps } from "./components";
export { KeyValueEditor } from "./components";
export type { KeyValueEditorProps, KeyValuePair } from "./components";
export { StatusBadge } from "./components";
export type { StatusBadgeProps, StatusVariant } from "./components";

// Settings
export type { SettingsAdapter, SettingEntry } from "./settings";
export { TauriSettingsAdapter } from "./settings";
export { SettingsProvider, useSettingsAdapter } from "./settings";
export { useSettings, type UseSettingsResult } from "./settings";

// Modules
export * from "./modules";

// Re-export Mantine for convenience — apps import from @uni-fw/ui instead of @mantine/core directly
export * from "@mantine/core";
export * from "@mantine/hooks";
export { Notifications, notifications } from "@mantine/notifications";

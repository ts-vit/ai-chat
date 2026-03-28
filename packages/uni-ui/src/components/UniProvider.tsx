import type { ReactNode } from "react";
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { uniTheme } from "../theme/uniTheme";
import { uniCssResolver } from "../theme/cssResolver";
import { SettingsProvider } from "../settings/SettingsContext";
import type { SettingsAdapter } from "../settings/types";
import "@mantine/core/styles.css";
import "@mantine/notifications/styles.css";

export interface UniProviderProps {
  children: ReactNode;
  /** Default color scheme. Defaults to "dark". */
  defaultColorScheme?: "light" | "dark" | "auto";
  /** Notification position. Defaults to "top-right". */
  notificationPosition?:
    | "top-right"
    | "top-left"
    | "bottom-right"
    | "bottom-left";
  /** Auto-close notifications (ms). Defaults to 4000. */
  notificationAutoClose?: number;
  /**
   * Settings adapter for @uni-fw/ui modules.
   * Use TauriSettingsAdapter for Tauri apps.
   * If not provided, useSettings() will throw — only omit if
   * you don't use any settings-dependent modules.
   */
  settingsAdapter?: SettingsAdapter;
}

/**
 * UNI theme provider — wraps MantineProvider + Notifications.
 * Drop-in replacement for manual MantineProvider setup.
 */
export function UniProvider({
  children,
  defaultColorScheme = "dark",
  notificationPosition = "top-right",
  notificationAutoClose = 4000,
  settingsAdapter,
}: UniProviderProps) {
  const content = settingsAdapter ? (
    <SettingsProvider adapter={settingsAdapter}>{children}</SettingsProvider>
  ) : (
    children
  );

  return (
    <MantineProvider
      theme={uniTheme}
      cssVariablesResolver={uniCssResolver}
      defaultColorScheme={defaultColorScheme}
    >
      <Notifications
        position={notificationPosition}
        autoClose={notificationAutoClose}
      />
      {content}
    </MantineProvider>
  );
}

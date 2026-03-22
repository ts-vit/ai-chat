import { createContext, useContext, type ReactNode } from "react";
import type { SettingsAdapter } from "./types";

const SettingsContext = createContext<SettingsAdapter | null>(null);

export interface SettingsProviderProps {
  adapter: SettingsAdapter;
  children: ReactNode;
}

export function SettingsProvider({ adapter, children }: SettingsProviderProps) {
  return (
    <SettingsContext.Provider value={adapter}>
      {children}
    </SettingsContext.Provider>
  );
}

/**
 * Access the settings adapter from any component inside UniProvider.
 * Throws if used outside UniProvider with settingsAdapter prop.
 */
export function useSettingsAdapter(): SettingsAdapter {
  const ctx = useContext(SettingsContext);
  if (!ctx) {
    throw new Error(
      "useSettingsAdapter: no SettingsAdapter found. " +
        "Make sure UniProvider has a settingsAdapter prop."
    );
  }
  return ctx;
}

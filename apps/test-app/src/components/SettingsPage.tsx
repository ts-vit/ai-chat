import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, ScrollArea, Stack } from "@mantine/core";
import { SshTunnelSettings } from "@uni-fw/ssh-ui";
import { TerminalSettings, OpenRouterSettings, OllamaSettings, GenerationSettings, InterfaceSettings, WebSearchSettings, BudgetSettings } from "@uni-fw/ui";
import {
  IconNetwork,
  IconTerminal2,
  IconKey,
  IconServer,
  IconAdjustments,
  IconPalette,
  IconWorldSearch,
  IconCurrencyDollar,
} from "@tabler/icons-react";

type SettingsSection = "ssh" | "terminal-settings" | "openRouter" | "ollama" | "generation" | "interface" | "webSearch" | "budget";

export function SettingsPage() {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingsSection>("ssh");

  const renderSection = () => {
    switch (activeSection) {
      case "ssh":
        return <SshTunnelSettings />;
      case "terminal-settings":
        return <TerminalSettings />;
      case "openRouter":
        return <OpenRouterSettings />;
      case "ollama":
        return <OllamaSettings />;
      case "generation":
        return <GenerationSettings />;
      case "interface":
        return <InterfaceSettings />;
      case "webSearch":
        return <WebSearchSettings />;
      case "budget":
        return <BudgetSettings />;
      default:
        return null;
    }
  };

  return (
    <div style={{ display: "flex", height: "100%" }}>
      <ScrollArea w={220} p="xs" style={{ borderRight: "1px solid var(--mantine-color-default-border)" }}>
        <Stack gap={2}>
          <NavLink
            label={t("settings.nav.vpn")}
            leftSection={<IconNetwork size={18} stroke={1.5} />}
            active={activeSection === "ssh"}
            onClick={() => setActiveSection("ssh")}
          />
          <NavLink
            label={t("settings.nav.terminal")}
            leftSection={<IconTerminal2 size={18} stroke={1.5} />}
            active={activeSection === "terminal-settings"}
            onClick={() => setActiveSection("terminal-settings")}
          />
          <NavLink
            label={t("settings.nav.openRouter")}
            leftSection={<IconKey size={18} stroke={1.5} />}
            active={activeSection === "openRouter"}
            onClick={() => setActiveSection("openRouter")}
          />
          <NavLink
            label={t("settings.nav.ollama")}
            leftSection={<IconServer size={18} stroke={1.5} />}
            active={activeSection === "ollama"}
            onClick={() => setActiveSection("ollama")}
          />
          <NavLink
            label={t("settings.nav.generation")}
            leftSection={<IconAdjustments size={18} stroke={1.5} />}
            active={activeSection === "generation"}
            onClick={() => setActiveSection("generation")}
          />
          <NavLink
            label={t("settings.nav.interface")}
            leftSection={<IconPalette size={18} stroke={1.5} />}
            active={activeSection === "interface"}
            onClick={() => setActiveSection("interface")}
          />
          <NavLink
            label={t("settings.nav.webSearch")}
            leftSection={<IconWorldSearch size={18} stroke={1.5} />}
            active={activeSection === "webSearch"}
            onClick={() => setActiveSection("webSearch")}
          />
          <NavLink
            label={t("budget.title")}
            leftSection={<IconCurrencyDollar size={18} stroke={1.5} />}
            active={activeSection === "budget"}
            onClick={() => setActiveSection("budget")}
          />
        </Stack>
      </ScrollArea>
      <ScrollArea flex={1} p="lg">
        {renderSection()}
      </ScrollArea>
    </div>
  );
}

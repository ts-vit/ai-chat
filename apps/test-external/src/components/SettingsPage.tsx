import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, ScrollArea, Stack } from "@mantine/core";
import { SshTunnelSettings } from "@uni-fw/ssh-ui";
import {
  IconNetwork,
} from "@tabler/icons-react";

type SettingsSection = "ssh";

export function SettingsPage() {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingsSection>("ssh");

  const renderSection = () => {
    switch (activeSection) {
      case "ssh":
        return <SshTunnelSettings />;
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
        </Stack>
      </ScrollArea>
      <ScrollArea flex={1} p="lg">
        {renderSection()}
      </ScrollArea>
    </div>
  );
}

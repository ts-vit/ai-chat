import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AppShell, Title, Text, Stack, Group, ActionIcon, Tooltip } from "@mantine/core";
import { IconSettings, IconTerminal2 } from "@tabler/icons-react";
import { SettingsPage } from "./components/SettingsPage";
import { TerminalPanel } from "@uni/terminal-ui";

type View = "main" | "settings";

export function App() {
  const { t } = useTranslation();
  const [view, setView] = useState<View>("main");
  const [terminalOpen, setTerminalOpen] = useState(false);
  const [terminalPosition, setTerminalPosition] = useState<"bottom" | "right">("bottom");

  return (
    <AppShell padding="md" header={{ height: 50 }}>
      <AppShell.Header>
        <Group h="100%" px="md" justify="space-between">
          <Text fw={600}>{t("app.title")}</Text>
          <Group gap="xs">
            <Tooltip label={t("terminal.tooltip")}>
              <ActionIcon variant="subtle" onClick={() => setTerminalOpen(o => !o)}>
                <IconTerminal2 size={20} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
            <Tooltip label={t("common.settings")}>
              <ActionIcon variant="subtle" onClick={() => setView(v => v === "settings" ? "main" : "settings")}>
                <IconSettings size={20} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          </Group>
        </Group>
      </AppShell.Header>
      <AppShell.Main>
        {view === "settings" ? (
          <SettingsPage />
        ) : (
          <Stack align="center" justify="center" h="100vh" gap="md">
            <Title order={1}>{t("app.title")}</Title>
            <Text c="dimmed">{t("app.description")}</Text>
          </Stack>
        )}
      </AppShell.Main>
      {terminalOpen && (
        <TerminalPanel
          position={terminalPosition}
          onPositionChange={setTerminalPosition}
          onClose={() => setTerminalOpen(false)}
          height={300}
        />
      )}
    </AppShell>
  );
}

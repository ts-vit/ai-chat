import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  Loader,
  Select,
  Slider,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useSettings } from "../../settings/useSettings";

const isWindows = navigator.userAgent.includes("Windows");

const SHELL_OPTIONS = isWindows
  ? [
      { value: "", label: "Auto" },
      { value: "pwsh.exe", label: "PowerShell 7 (pwsh)" },
      { value: "powershell.exe", label: "PowerShell" },
      { value: "cmd.exe", label: "CMD" },
      { value: "__custom__", label: "Custom..." },
    ]
  : [
      { value: "", label: "Auto" },
      { value: "/bin/bash", label: "bash" },
      { value: "/bin/zsh", label: "zsh" },
      { value: "/bin/sh", label: "sh" },
      { value: "__custom__", label: "Custom..." },
    ];

const knownValues = SHELL_OPTIONS.map((o) => o.value);

export function TerminalSettings() {
  const { t } = useTranslation();

  const fontSizeSetting = useSettings("terminal.font_size");
  const shellSetting = useSettings("terminal.shell");

  const [fontSizeLocal, setFontSizeLocal] = useState(13);
  const [customShell, setCustomShell] = useState("");

  const shell = shellSetting.value ?? "";
  const isCustom = shell !== "" && !knownValues.includes(shell);
  const [showCustom, setShowCustom] = useState(false);

  useEffect(() => {
    setFontSizeLocal(parseInt(fontSizeSetting.value ?? "13") || 13);
  }, [fontSizeSetting.value]);

  useEffect(() => {
    if (isCustom) {
      setShowCustom(true);
      setCustomShell(shell);
    }
  }, [isCustom, shell]);

  const prevRef = useRef<Record<string, string>>({});

  const showSaved = useCallback(
    (key: string, value: string) => {
      if (prevRef.current[key] === value) return;
      prevRef.current[key] = value;
      notifications.show({
        message: t("terminal.saved"),
        color: "green",
        autoClose: 2000,
      });
    },
    [t],
  );

  const isLoading = fontSizeSetting.loading || shellSetting.loading;

  if (isLoading) {
    return <Loader size="sm" />;
  }

  const selectValue = isCustom ? "__custom__" : shell;

  return (
    <Stack gap="lg">
      <Title order={4}>{t("terminal.title")}</Title>

      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("terminal.fontSize")}
        </Text>
        <Slider
          value={fontSizeLocal}
          onChange={setFontSizeLocal}
          onChangeEnd={(v) => {
            fontSizeSetting.set(String(v));
            showSaved("fontSize", String(v));
          }}
          min={10}
          max={20}
          step={1}
          marks={[
            { value: 10, label: "10" },
            { value: 13, label: "13" },
            { value: 16, label: "16" },
            { value: 20, label: "20" },
          ]}
        />
      </Stack>

      <Select
        label={t("terminal.shell")}
        value={selectValue}
        onChange={(value) => {
          if (value === "__custom__") {
            setShowCustom(true);
            setCustomShell("");
            shellSetting.set("");
            showSaved("shell", "__custom__");
          } else {
            setShowCustom(false);
            const v = value ?? "";
            shellSetting.set(v);
            showSaved("shell", v);
          }
        }}
        data={SHELL_OPTIONS.map((o) => ({
          ...o,
          label: o.value === "" ? t("terminal.shellAuto") : o.label,
        }))}
      />

      {showCustom && (
        <TextInput
          label={t("terminal.shell")}
          placeholder={isWindows ? "C:\\path\\to\\shell.exe" : "/usr/bin/fish"}
          value={customShell}
          onChange={(e) => setCustomShell(e.currentTarget.value)}
          onBlur={() => {
            shellSetting.set(customShell);
            showSaved("customShell", customShell);
          }}
        />
      )}
    </Stack>
  );
}

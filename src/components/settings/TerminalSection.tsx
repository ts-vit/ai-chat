import { useTranslation } from "react-i18next";
import { Select, Slider, Stack, Text, TextInput, Title } from "@mantine/core";
import { useState } from "react";

interface Props {
    terminalFontSize: number;
    onTerminalFontSizeChange: (value: number) => void;
    terminalShell: string;
    onTerminalShellChange: (value: string) => void;
}

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

export function TerminalSection({
    terminalFontSize,
    onTerminalFontSizeChange,
    terminalShell,
    onTerminalShellChange,
}: Props) {
    const { t } = useTranslation();
    const knownValues = SHELL_OPTIONS.map((o) => o.value);
    const isCustom = terminalShell !== "" && !knownValues.includes(terminalShell);
    const [showCustom, setShowCustom] = useState(isCustom);

    const selectValue = isCustom ? "__custom__" : terminalShell;

    return (
        <Stack gap="lg">
            <Title order={4}>{t("terminal.title")}</Title>

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("terminal.fontSize")}
                </Text>
                <Slider
                    value={terminalFontSize}
                    onChange={onTerminalFontSizeChange}
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
                        onTerminalShellChange("");
                    } else {
                        setShowCustom(false);
                        onTerminalShellChange(value ?? "");
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
                    value={terminalShell}
                    onChange={(e) => onTerminalShellChange(e.currentTarget.value)}
                />
            )}
        </Stack>
    );
}

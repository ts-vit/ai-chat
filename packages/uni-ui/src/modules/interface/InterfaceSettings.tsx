import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  Checkbox,
  Paper,
  SegmentedControl,
  Select,
  Slider,
  Stack,
  Switch,
  Text,
  useMantineColorScheme,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useSettings } from "../../settings/useSettings";

const STATUS_BAR_METRIC_IDS = ["balance", "context", "tokens", "cost"] as const;

const LANGUAGE_OPTIONS = [
  { value: "", labelKey: "settings.interface.languageSystem" as const },
  { value: "ru", labelKey: "settings.interface.languageRu" as const },
  { value: "en", labelKey: "settings.interface.languageEn" as const },
];

function getSystemLanguage(): string {
  return navigator.language.startsWith("ru") ? "ru" : "en";
}

export function InterfaceSettings() {
  const { t, i18n } = useTranslation();
  const { colorScheme, setColorScheme } = useMantineColorScheme();

  const languageSetting = useSettings("ui.language");
  const fontSizeSetting = useSettings("ui.font_size");
  const sendByEnterSetting = useSettings("ui.send_by_enter");
  const densitySetting = useSettings("ui.message_density");
  const chatWidthSetting = useSettings("ui.chat_width");
  const showStatusBarSetting = useSettings("ui.show_status_bar");
  const metricsSetting = useSettings("ui.status_bar_metrics");

  const [fontSizeLocal, setFontSizeLocal] = useState(14);

  const prevRef = useRef<Record<string, string>>({});

  // Sync fontSize from settings on load
  useEffect(() => {
    if (fontSizeSetting.value !== undefined) {
      setFontSizeLocal(
        fontSizeSetting.value !== null
          ? parseInt(fontSizeSetting.value) || 14
          : 14
      );
    }
  }, [fontSizeSetting.value]);

  const showSaved = useCallback(
    (key: string, value: string) => {
      if (prevRef.current[key] === value) return;
      prevRef.current[key] = value;
      notifications.show({
        message: t("settings.interface.saved"),
        color: "green",
        autoClose: 2000,
      });
    },
    [t]
  );

  // Derived values
  const language = languageSetting.value ?? "";
  const sendByEnter = sendByEnterSetting.value !== "false";
  const messageDensity = densitySetting.value ?? "standard";
  const chatWidth = chatWidthSetting.value ?? "standard";
  const showStatusBar = showStatusBarSetting.value !== "false";
  const statusBarMetrics: string[] = (() => {
    try {
      return JSON.parse(
        metricsSetting.value ?? '["balance","context","tokens","cost"]'
      );
    } catch {
      return ["balance", "context", "tokens", "cost"];
    }
  })();

  const themeValue = colorScheme === "auto" ? "dark" : colorScheme;

  const handleLanguageChange = (value: string | null) => {
    const next = value ?? "";
    languageSetting.set(next);
    i18n.changeLanguage(next || getSystemLanguage());
    showSaved("language", next);
  };

  return (
    <Stack gap="lg">
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.interface.theme")}
        </Text>
        <SegmentedControl
          value={themeValue}
          onChange={(v) => setColorScheme(v as "light" | "dark")}
          data={[
            { label: t("settings.interface.themeDark"), value: "dark" },
            { label: t("settings.interface.themeLight"), value: "light" },
          ]}
        />
      </Stack>
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.interface.messageDensity")}
        </Text>
        <SegmentedControl
          value={messageDensity}
          onChange={(v) => {
            densitySetting.set(v);
            showSaved("density", v);
          }}
          data={[
            {
              label: t("settings.interface.densityCompact"),
              value: "compact",
            },
            {
              label: t("settings.interface.densityStandard"),
              value: "standard",
            },
            {
              label: t("settings.interface.densitySpacious"),
              value: "spacious",
            },
          ]}
        />
      </Stack>
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.interface.chatWidth")}
        </Text>
        <SegmentedControl
          value={chatWidth}
          onChange={(v) => {
            chatWidthSetting.set(v);
            showSaved("chatWidth", v);
          }}
          data={[
            {
              label: t("settings.interface.chatWidthNarrow"),
              value: "narrow",
            },
            {
              label: t("settings.interface.chatWidthStandard"),
              value: "standard",
            },
            { label: t("settings.interface.chatWidthWide"), value: "wide" },
          ]}
        />
      </Stack>
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.interface.sendMethod")}
        </Text>
        <SegmentedControl
          value={sendByEnter ? "true" : "false"}
          onChange={(v) => {
            const val = v === "true" ? "true" : "false";
            sendByEnterSetting.set(val);
            showSaved("sendByEnter", val);
          }}
          data={[
            {
              label: t("settings.interface.sendByEnter"),
              value: "true",
            },
            {
              label: t("settings.interface.sendByCtrlEnter"),
              value: "false",
            },
          ]}
        />
      </Stack>
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.interface.language")}
        </Text>
        <Select
          value={language}
          onChange={handleLanguageChange}
          data={LANGUAGE_OPTIONS.map((o) => ({
            value: o.value,
            label: t(o.labelKey),
          }))}
          allowDeselect={false}
        />
      </Stack>
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.interface.fontSize")}
        </Text>
        <Text size="sm" mb={4}>
          {fontSizeLocal}px
        </Text>
        <Slider
          min={12}
          max={24}
          step={1}
          value={fontSizeLocal}
          onChange={setFontSizeLocal}
          onChangeEnd={(v) => {
            fontSizeSetting.set(String(v));
            showSaved("fontSize", String(v));
          }}
          marks={[
            { value: 12, label: "12" },
            { value: 14, label: "14" },
            { value: 16, label: "16" },
            { value: 18, label: "18" },
            { value: 20, label: "20" },
            { value: 24, label: "24" },
          ]}
        />
        <Paper p="sm" withBorder>
          <Text style={{ fontSize: `${fontSizeLocal}px` }}>
            {t("settings.interface.fontPreview")}
          </Text>
        </Paper>
      </Stack>
      <Stack gap="xs">
        <Switch
          label={t("settings.interface.showStatusBar")}
          checked={showStatusBar}
          onChange={(e) => {
            const val = e.currentTarget.checked ? "true" : "false";
            showStatusBarSetting.set(val);
            showSaved("showStatusBar", val);
          }}
        />
        {showStatusBar && (
          <Checkbox.Group
            label={t("settings.interface.statusBarMetrics")}
            value={statusBarMetrics}
            onChange={(values) => {
              const json = JSON.stringify(values);
              metricsSetting.set(json);
              showSaved("metrics", json);
            }}
          >
            <Stack gap="xs" mt="xs">
              {STATUS_BAR_METRIC_IDS.map((id) => (
                <Checkbox
                  key={id}
                  value={id}
                  label={t(
                    `settings.interface.metric${id.charAt(0).toUpperCase() + id.slice(1)}`
                  )}
                />
              ))}
            </Stack>
          </Checkbox.Group>
        )}
      </Stack>
    </Stack>
  );
}

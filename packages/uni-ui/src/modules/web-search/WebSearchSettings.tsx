import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  Loader,
  PasswordInput,
  Select,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useSettings } from "../../settings/useSettings";

const PROVIDER_OPTIONS = [
  { value: "", label: "—" },
  { value: "tavily", label: "Tavily" },
  { value: "brave", label: "Brave Search" },
  { value: "uni", label: "UNI Search" },
];

export function WebSearchSettings() {
  const { t } = useTranslation();

  const providerSetting = useSettings("search.provider");
  const tavilyKeySetting = useSettings("search.tavily.api_key");
  const braveKeySetting = useSettings("search.brave.api_key");

  const provider = providerSetting.value ?? "";

  const [tavilyKeyLocal, setTavilyKeyLocal] = useState("");
  const [braveKeyLocal, setBraveKeyLocal] = useState("");

  const prevRef = useRef<Record<string, string>>({});

  useEffect(() => {
    if (tavilyKeySetting.value !== undefined) {
      setTavilyKeyLocal(tavilyKeySetting.value ?? "");
    }
  }, [tavilyKeySetting.value]);

  useEffect(() => {
    if (braveKeySetting.value !== undefined) {
      setBraveKeyLocal(braveKeySetting.value ?? "");
    }
  }, [braveKeySetting.value]);

  const showSaved = useCallback(
    (key: string, value: string) => {
      if (prevRef.current[key] === value) return;
      prevRef.current[key] = value;
      notifications.show({
        message: t("settings.webSearch.saved"),
        color: "green",
        autoClose: 2000,
      });
    },
    [t],
  );

  const isLoading =
    providerSetting.loading ||
    tavilyKeySetting.loading ||
    braveKeySetting.loading;

  if (isLoading) {
    return <Loader size="sm" />;
  }

  return (
    <Stack gap="lg">
      <Title order={4}>{t("settings.webSearch.title")}</Title>

      <Select
        label={t("settings.webSearch.provider")}
        value={provider}
        onChange={(value) => {
          const v = value ?? "";
          providerSetting.set(v);
          showSaved("provider", v);
        }}
        data={PROVIDER_OPTIONS}
      />

      {provider === "tavily" && (
        <Stack gap="sm">
          <PasswordInput
            label={t("settings.webSearch.tavilyApiKey")}
            value={tavilyKeyLocal}
            onChange={(e) => setTavilyKeyLocal(e.currentTarget.value)}
            onBlur={() => {
              if (tavilyKeyLocal !== (tavilyKeySetting.value ?? "")) {
                tavilyKeySetting.set(tavilyKeyLocal);
                showSaved("tavilyKey", tavilyKeyLocal);
              }
            }}
          />
          <Text size="xs" c="dimmed">
            {t("settings.webSearch.tavilyHint")}
          </Text>
        </Stack>
      )}

      {provider === "brave" && (
        <Stack gap="sm">
          <PasswordInput
            label={t("settings.webSearch.braveApiKey")}
            value={braveKeyLocal}
            onChange={(e) => setBraveKeyLocal(e.currentTarget.value)}
            onBlur={() => {
              if (braveKeyLocal !== (braveKeySetting.value ?? "")) {
                braveKeySetting.set(braveKeyLocal);
                showSaved("braveKey", braveKeyLocal);
              }
            }}
          />
          <Text size="xs" c="dimmed">
            {t("settings.webSearch.braveHint")}
          </Text>
        </Stack>
      )}

      {provider === "uni" && (
        <Text size="xs" c="dimmed">
          {t("settings.webSearch.uniDescription")}
        </Text>
      )}
    </Stack>
  );
}

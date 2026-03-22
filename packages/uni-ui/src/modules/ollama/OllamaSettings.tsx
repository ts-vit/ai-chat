import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Stack, Text, TextInput } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useSettings } from "../../settings/useSettings";
import { useOllamaApi } from "./useOllamaApi";
import { OllamaModels } from "./OllamaModels";

const DEFAULT_URL = "http://localhost:11434/v1";

export function OllamaSettings() {
  const { t } = useTranslation();
  const urlSetting = useSettings("llm.ollama.url");
  const selectedModelsSetting = useSettings("llm.ollama.models");

  const baseUrl = urlSetting.value ?? DEFAULT_URL;
  const selectedIds: string[] = (() => {
    try {
      return JSON.parse(selectedModelsSetting.value || "[]");
    } catch {
      return [];
    }
  })();

  const [urlLocal, setUrlLocal] = useState(baseUrl);

  useEffect(() => {
    setUrlLocal(urlSetting.value ?? DEFAULT_URL);
  }, [urlSetting.value]);

  const api = useOllamaApi(baseUrl);

  const showSaved = useCallback(() => {
    notifications.show({
      message: t("ollama.saved"),
      color: "green",
      autoClose: 2000,
    });
  }, [t]);

  useEffect(() => {
    api.checkStatus();
  }, [baseUrl]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (api.status === "available") {
      api.loadModels();
    }
  }, [api.status]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleToggleModel = (name: string) => {
    const newIds = selectedIds.includes(name)
      ? selectedIds.filter((id) => id !== name)
      : [...selectedIds, name];
    selectedModelsSetting.set(JSON.stringify(newIds));
    showSaved();
  };

  const handleUrlBlur = () => {
    if (urlLocal !== (urlSetting.value ?? DEFAULT_URL)) {
      urlSetting.set(urlLocal);
      showSaved();
    }
  };

  return (
    <Stack gap="lg">
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.ollama.url")}
        </Text>
        <TextInput
          placeholder={t("ollama.serverUrlPlaceholder")}
          value={urlLocal}
          onChange={(e) => setUrlLocal(e.currentTarget.value)}
          onBlur={handleUrlBlur}
        />
        <Text size="xs" c="dimmed">
          {t("settings.ollama.urlHint")}
        </Text>
      </Stack>

      <OllamaModels
        status={api.status}
        models={api.models}
        pullProgress={api.pullProgress}
        selectedIds={selectedIds}
        onToggleModel={handleToggleModel}
        onCheckStatus={api.checkStatus}
        onPullModel={api.pullModel}
        onDeleteModel={api.deleteModel}
        onCancelPull={api.cancelPull}
      />
    </Stack>
  );
}

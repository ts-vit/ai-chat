import { useCallback, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  Accordion,
  PasswordInput,
  Stack,
  Text,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useSettings } from "../../settings/useSettings";
import { useOpenRouterModels } from "./useOpenRouterModels";
import { ModelCatalog } from "./ModelCatalog";

export function OpenRouterSettings() {
  const { t } = useTranslation();
  const apiKey = useSettings("llm.openrouter.api_key");
  const mgmtKey = useSettings("llm.openrouter.mgmt_key");
  const selectedModels = useSettings("llm.openrouter.models");
  const { models, loading, error, reload } = useOpenRouterModels();

  const apiKeyDraft = useRef("");
  const mgmtKeyDraft = useRef("");

  const selectedIds: string[] = useMemo(() => {
    if (!selectedModels.value) return [];
    try {
      const parsed = JSON.parse(selectedModels.value);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }, [selectedModels.value]);

  const showSaved = useCallback(() => {
    notifications.show({
      message: t("settings.openrouter.saved"),
      color: "green",
      autoClose: 2000,
    });
  }, [t]);

  const handleApiKeyBlur = useCallback(async () => {
    const val = apiKeyDraft.current;
    if (val !== (apiKey.value ?? "")) {
      await apiKey.set(val);
      showSaved();
    }
  }, [apiKey, showSaved]);

  const handleMgmtKeyBlur = useCallback(async () => {
    const val = mgmtKeyDraft.current;
    if (val !== (mgmtKey.value ?? "")) {
      await mgmtKey.set(val);
      showSaved();
    }
  }, [mgmtKey, showSaved]);

  const handleToggleModel = useCallback(async (id: string) => {
    const current = selectedIds;
    const next = current.includes(id)
      ? current.filter((x) => x !== id)
      : [...current, id];
    await selectedModels.set(JSON.stringify(next));
    showSaved();
  }, [selectedIds, selectedModels, showSaved]);

  // Initialize draft refs when values load
  if (apiKey.value !== undefined && apiKey.value !== null && apiKeyDraft.current === "") {
    apiKeyDraft.current = apiKey.value;
  }
  if (mgmtKey.value !== undefined && mgmtKey.value !== null && mgmtKeyDraft.current === "") {
    mgmtKeyDraft.current = mgmtKey.value;
  }

  return (
    <Stack gap="lg">
      <Accordion variant="separated">
        <Accordion.Item value="api-key">
          <Accordion.Control>{t("settings.openrouter.apiKey")}</Accordion.Control>
          <Accordion.Panel>
            <Stack gap="xs">
              <PasswordInput
                placeholder="sk-or-..."
                defaultValue={apiKey.value ?? ""}
                key={apiKey.value === undefined ? "loading" : "loaded"}
                onChange={(e) => { apiKeyDraft.current = e.currentTarget.value; }}
                onBlur={handleApiKeyBlur}
              />
              <Text size="xs" c="dimmed">
                {t("settings.openrouter.apiKeyHint")}
              </Text>
            </Stack>
          </Accordion.Panel>
        </Accordion.Item>
        <Accordion.Item value="management-key">
          <Accordion.Control>{t("settings.openrouter.managementKey")}</Accordion.Control>
          <Accordion.Panel>
            <Stack gap="xs">
              <PasswordInput
                placeholder="sk-or-..."
                defaultValue={mgmtKey.value ?? ""}
                key={mgmtKey.value === undefined ? "loading" : "loaded"}
                onChange={(e) => { mgmtKeyDraft.current = e.currentTarget.value; }}
                onBlur={handleMgmtKeyBlur}
              />
              <Text size="xs" c="dimmed">
                {t("settings.openrouter.managementKeyHint")}
              </Text>
            </Stack>
          </Accordion.Panel>
        </Accordion.Item>
      </Accordion>

      <Stack gap="xs">
        <ModelCatalog
          models={models}
          loading={loading}
          error={error}
          onReload={() => reload(true)}
          selectedIds={selectedIds}
          onToggleModel={handleToggleModel}
        />
      </Stack>
    </Stack>
  );
}

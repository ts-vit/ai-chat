import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  Checkbox,
  Group,
  Loader,
  NumberInput,
  Slider,
  Stack,
  Text,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useSettings } from "../../settings/useSettings";

const DEFAULTS = {
  temperature: 0.7,
  maxTokens: 4096,
  topP: 0.9,
  topK: 40,
  frequencyPenalty: 0,
  presencePenalty: 0,
};

export function GenerationSettings() {
  const { t } = useTranslation();

  const tempSetting = useSettings("llm.temperature");
  const maxTokensSetting = useSettings("llm.max_tokens");
  const topPSetting = useSettings("llm.top_p");
  const topKSetting = useSettings("llm.top_k");
  const freqPenSetting = useSettings("llm.frequency_penalty");
  const presPenSetting = useSettings("llm.presence_penalty");

  // Local state for smooth slider/input interaction
  const [tempLocal, setTempLocal] = useState(DEFAULTS.temperature);
  const [maxTokensLocal, setMaxTokensLocal] = useState<number | string>(DEFAULTS.maxTokens);
  const [topPLocal, setTopPLocal] = useState(DEFAULTS.topP);
  const [topKLocal, setTopKLocal] = useState<number | string>(DEFAULTS.topK);
  const [freqPenLocal, setFreqPenLocal] = useState(DEFAULTS.frequencyPenalty);
  const [presPenLocal, setPresPenLocal] = useState(DEFAULTS.presencePenalty);

  // Track previous values to avoid unnecessary toasts
  const prevRef = useRef<Record<string, string>>({});

  // Sync local state from settings on load
  useEffect(() => {
    if (tempSetting.value !== undefined) {
      setTempLocal(tempSetting.value !== null ? parseFloat(tempSetting.value) || DEFAULTS.temperature : DEFAULTS.temperature);
    }
  }, [tempSetting.value]);

  useEffect(() => {
    if (maxTokensSetting.value !== undefined) {
      setMaxTokensLocal(maxTokensSetting.value !== null ? parseInt(maxTokensSetting.value) || DEFAULTS.maxTokens : DEFAULTS.maxTokens);
    }
  }, [maxTokensSetting.value]);

  useEffect(() => {
    if (topPSetting.value !== undefined && topPSetting.value !== null) {
      setTopPLocal(parseFloat(topPSetting.value) || DEFAULTS.topP);
    }
  }, [topPSetting.value]);

  useEffect(() => {
    if (topKSetting.value !== undefined && topKSetting.value !== null) {
      setTopKLocal(parseInt(topKSetting.value) || DEFAULTS.topK);
    }
  }, [topKSetting.value]);

  useEffect(() => {
    if (freqPenSetting.value !== undefined && freqPenSetting.value !== null) {
      setFreqPenLocal(parseFloat(freqPenSetting.value) ?? DEFAULTS.frequencyPenalty);
    }
  }, [freqPenSetting.value]);

  useEffect(() => {
    if (presPenSetting.value !== undefined && presPenSetting.value !== null) {
      setPresPenLocal(parseFloat(presPenSetting.value) ?? DEFAULTS.presencePenalty);
    }
  }, [presPenSetting.value]);

  const showSaved = useCallback(
    (key: string, value: string) => {
      if (prevRef.current[key] === value) return;
      prevRef.current[key] = value;
      notifications.show({
        message: t("settings.generation.saved"),
        color: "green",
        autoClose: 2000,
      });
    },
    [t]
  );

  const isLoading =
    tempSetting.loading ||
    maxTokensSetting.loading ||
    topPSetting.loading ||
    topKSetting.loading ||
    freqPenSetting.loading ||
    presPenSetting.loading;

  if (isLoading) {
    return <Loader size="sm" />;
  }

  const topPEnabled = topPSetting.value !== null;
  const topKEnabled = topKSetting.value !== null;
  const freqPenEnabled = freqPenSetting.value !== null;
  const presPenEnabled = presPenSetting.value !== null;

  return (
    <Stack gap="xl">
      <Text size="sm" fw={500}>
        {t("settings.generation.title")}
      </Text>

      {/* Temperature */}
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.generation.temperature")}
        </Text>
        <Text size="xs" c="dimmed">
          {t("settings.generation.temperatureDescription")}
        </Text>
        <Slider
          min={0}
          max={2}
          step={0.1}
          value={tempLocal}
          onChange={setTempLocal}
          onChangeEnd={(v) => {
            const s = String(v);
            tempSetting.set(s);
            showSaved("temperature", s);
          }}
        />
      </Stack>

      {/* Max Tokens */}
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          {t("settings.generation.maxTokens")}
        </Text>
        <Text size="xs" c="dimmed">
          {t("settings.generation.maxTokensDescription")}
        </Text>
        <NumberInput
          value={maxTokensLocal}
          onChange={(val) => setMaxTokensLocal(val)}
          onBlur={() => {
            const num = Number(maxTokensLocal) || DEFAULTS.maxTokens;
            const s = String(num);
            maxTokensSetting.set(s);
            showSaved("maxTokens", s);
          }}
          min={1}
          max={200000}
          style={{ maxWidth: 200 }}
        />
      </Stack>

      {/* Top P */}
      <Stack gap="xs">
        <Group justify="space-between" align="center">
          <Text size="sm" fw={500}>
            {t("settings.generation.topP")}
          </Text>
          <Checkbox
            label={t("settings.generation.enable")}
            checked={topPEnabled}
            onChange={(e) => {
              if (e.currentTarget.checked) {
                topPSetting.set(String(DEFAULTS.topP));
                setTopPLocal(DEFAULTS.topP);
              } else {
                topPSetting.delete();
              }
              showSaved("topP.enabled", String(e.currentTarget.checked));
            }}
          />
        </Group>
        <Text size="xs" c="dimmed">
          {t("settings.generation.topPDescription")}
        </Text>
        {topPEnabled && (
          <Slider
            min={0}
            max={1}
            step={0.05}
            value={topPLocal}
            onChange={setTopPLocal}
            onChangeEnd={(v) => {
              const s = String(v);
              topPSetting.set(s);
              showSaved("topP", s);
            }}
          />
        )}
      </Stack>

      {/* Top K */}
      <Stack gap="xs">
        <Group justify="space-between" align="center">
          <Text size="sm" fw={500}>
            {t("settings.generation.topK")}
          </Text>
          <Checkbox
            label={t("settings.generation.enable")}
            checked={topKEnabled}
            onChange={(e) => {
              if (e.currentTarget.checked) {
                topKSetting.set(String(DEFAULTS.topK));
                setTopKLocal(DEFAULTS.topK);
              } else {
                topKSetting.delete();
              }
              showSaved("topK.enabled", String(e.currentTarget.checked));
            }}
          />
        </Group>
        <Text size="xs" c="dimmed">
          {t("settings.generation.topKDescription")}
        </Text>
        {topKEnabled && (
          <NumberInput
            value={topKLocal}
            onChange={(val) => setTopKLocal(val)}
            onBlur={() => {
              const num = Number(topKLocal) || DEFAULTS.topK;
              const s = String(num);
              topKSetting.set(s);
              showSaved("topK", s);
            }}
            min={1}
            max={500}
            style={{ maxWidth: 140 }}
          />
        )}
      </Stack>

      {/* Frequency Penalty */}
      <Stack gap="xs">
        <Group justify="space-between" align="center">
          <Text size="sm" fw={500}>
            {t("settings.generation.frequencyPenalty")}
          </Text>
          <Checkbox
            label={t("settings.generation.enable")}
            checked={freqPenEnabled}
            onChange={(e) => {
              if (e.currentTarget.checked) {
                freqPenSetting.set(String(DEFAULTS.frequencyPenalty));
                setFreqPenLocal(DEFAULTS.frequencyPenalty);
              } else {
                freqPenSetting.delete();
              }
              showSaved("freqPen.enabled", String(e.currentTarget.checked));
            }}
          />
        </Group>
        <Text size="xs" c="dimmed">
          {t("settings.generation.frequencyPenaltyDescription")}
        </Text>
        {freqPenEnabled && (
          <Slider
            min={-2}
            max={2}
            step={0.1}
            value={freqPenLocal}
            onChange={setFreqPenLocal}
            onChangeEnd={(v) => {
              const s = String(v);
              freqPenSetting.set(s);
              showSaved("freqPen", s);
            }}
          />
        )}
      </Stack>

      {/* Presence Penalty */}
      <Stack gap="xs">
        <Group justify="space-between" align="center">
          <Text size="sm" fw={500}>
            {t("settings.generation.presencePenalty")}
          </Text>
          <Checkbox
            label={t("settings.generation.enable")}
            checked={presPenEnabled}
            onChange={(e) => {
              if (e.currentTarget.checked) {
                presPenSetting.set(String(DEFAULTS.presencePenalty));
                setPresPenLocal(DEFAULTS.presencePenalty);
              } else {
                presPenSetting.delete();
              }
              showSaved("presPen.enabled", String(e.currentTarget.checked));
            }}
          />
        </Group>
        <Text size="xs" c="dimmed">
          {t("settings.generation.presencePenaltyDescription")}
        </Text>
        {presPenEnabled && (
          <Slider
            min={-2}
            max={2}
            step={0.1}
            value={presPenLocal}
            onChange={setPresPenLocal}
            onChangeEnd={(v) => {
              const s = String(v);
              presPenSetting.set(s);
              showSaved("presPen", s);
            }}
          />
        )}
      </Stack>
    </Stack>
  );
}

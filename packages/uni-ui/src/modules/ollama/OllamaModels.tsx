import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ActionIcon,
  Button,
  Checkbox,
  Group,
  Loader,
  Progress,
  Select,
  Stack,
  Text,
  ThemeIcon,
  Tooltip,
} from "@mantine/core";
import { IconCircleCheck, IconCircleX, IconTrash } from "@tabler/icons-react";
import { ConfirmModal } from "../../components/ConfirmModal";
import type { OllamaModel, OllamaStatus, PullProgress } from "./types";

const PULL_MODEL_KEYS = [
  { value: "qwen2.5:0.5b", labelKey: "ollama.pullQwen05" as const },
  { value: "qwen2.5:1.5b", labelKey: "ollama.pullQwen15" as const },
  { value: "gemma2:2b", labelKey: "ollama.pullGemma" as const },
  { value: "qwen2.5:3b", labelKey: "ollama.pullQwen3" as const },
  { value: "qwen2.5-coder:3b", labelKey: "ollama.pullQwenCoder" as const },
  { value: "llama3.2:3b", labelKey: "ollama.pullLlama" as const },
  { value: "phi3:latest", labelKey: "ollama.pullPhi" as const },
] as const;

interface OllamaModelsProps {
  status: OllamaStatus;
  models: OllamaModel[];
  pullProgress: PullProgress | null;
  selectedIds: string[];
  onToggleModel: (name: string) => void;
  onCheckStatus: () => void;
  onPullModel: (name: string) => Promise<void>;
  onDeleteModel: (name: string) => Promise<void>;
  onCancelPull: () => void;
}

export function OllamaModels({
  status,
  models,
  pullProgress,
  selectedIds,
  onToggleModel,
  onPullModel,
  onDeleteModel,
  onCancelPull,
}: OllamaModelsProps) {
  const { t } = useTranslation();
  const [deletingModelName, setDeletingModelName] = useState<string | null>(
    null
  );
  const [selectedModel, setSelectedModel] = useState<string>("qwen2.5:3b");

  const isPulling = pullProgress !== null;
  const isModelInstalled = models.some((m) => m.name === selectedModel);

  const handlePull = () => {
    onPullModel(selectedModel);
  };

  const handleConfirmDelete = () => {
    if (deletingModelName) {
      onDeleteModel(deletingModelName);
      setDeletingModelName(null);
    }
  };

  return (
    <Stack gap="xs">
      <Text size="sm" fw={500}>
        {t("ollama.title")}
      </Text>

      {/* Status */}
      <Group gap="xs">
        {status === "unknown" || status === "checking" ? (
          <>
            <Loader size="xs" />
            <Text size="sm">{t("ollama.checking")}</Text>
          </>
        ) : null}
        {status === "available" && (
          <>
            <ThemeIcon size="sm" color="green">
              <IconCircleCheck size={16} stroke={1.5} />
            </ThemeIcon>
            <Text size="sm">{t("ollama.running")}</Text>
          </>
        )}
        {status === "unavailable" && (
          <>
            <IconCircleX
              size={18}
              stroke={1.5}
              color="var(--mantine-color-red-6)"
            />
            <Text size="sm">{t("ollama.notFound")}</Text>
            <Button
              variant="light"
              size="xs"
              onClick={() => window.open("https://ollama.com")}
            >
              {t("ollama.installOllama")}
            </Button>
          </>
        )}
      </Group>

      {status === "available" && (
        <>
          {/* Installed models */}
          <Text size="sm" fw={500} mt="xs">
            {t("ollama.installedModels")}
          </Text>
          {models.length === 0 ? (
            <Text size="sm" c="dimmed">
              {t("ollama.noInstalledModels")}
            </Text>
          ) : (
            <Stack gap="xs">
              <Text size="xs" c="dimmed">
                {t("ollama.selectedCount", { count: selectedIds.length })}
              </Text>
              {models.map((m) => {
                const checked = selectedIds.includes(m.name);
                return (
                  <Group key={m.name} justify="space-between">
                    <Group gap="xs">
                      <Checkbox
                        checked={checked}
                        onChange={() => onToggleModel(m.name)}
                        label={t("ollama.showWhenSelectingChat")}
                        size="xs"
                      />
                      <Text size="sm" fw={500}>
                        {m.name}
                      </Text>
                      <Text size="sm" c="dimmed">
                        {m.size}
                      </Text>
                    </Group>
                    <Tooltip label={t("ollama.deleteModel")}>
                      <ActionIcon
                        size="xs"
                        variant="subtle"
                        onClick={() => setDeletingModelName(m.name)}
                        aria-label={t("ollama.deleteModel")}
                      >
                        <IconTrash size={16} stroke={1.5} />
                      </ActionIcon>
                    </Tooltip>
                  </Group>
                );
              })}
            </Stack>
          )}

          {/* Download model */}
          <Text size="sm" fw={500} mt="xs">
            {t("ollama.downloadModel")}
          </Text>
          <Group align="flex-end" gap="sm">
            <Select
              label={null}
              data={PULL_MODEL_KEYS.map((p) => ({
                value: p.value,
                label: t(p.labelKey),
              }))}
              value={selectedModel}
              onChange={(v) => v && setSelectedModel(v)}
              allowDeselect={false}
              style={{ minWidth: 350 }}
              styles={{ dropdown: { minWidth: 350 } }}
            />
            <Button
              variant="filled"
              onClick={handlePull}
              disabled={isModelInstalled || isPulling}
            >
              {t("ollama.download")}
            </Button>
          </Group>

          {pullProgress && (
            <Stack gap="xs">
              <Progress value={pullProgress.progress} size="sm" animated />
              <Text size="xs" c="dimmed">
                {pullProgress.status || t("common.loading")}
              </Text>
              <Button variant="subtle" size="xs" onClick={onCancelPull}>
                {t("ollama.cancel")}
              </Button>
            </Stack>
          )}
        </>
      )}

      <ConfirmModal
        opened={deletingModelName !== null}
        onClose={() => setDeletingModelName(null)}
        onConfirm={handleConfirmDelete}
        title={t("ollama.deleteModel")}
        message={
          deletingModelName
            ? t("ollama.deleteModelConfirm", { name: deletingModelName })
            : ""
        }
      />
    </Stack>
  );
}

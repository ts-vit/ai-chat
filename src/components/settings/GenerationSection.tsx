import { useTranslation } from "react-i18next";
import {
    Checkbox,
    Group,
    NumberInput,
    Slider,
    Stack,
    Text,
} from "@mantine/core";

export interface GenerationSectionProps {
    temperature: number;
    onTemperatureChange: (t: number) => void;
    maxTokens: number;
    onMaxTokensChange: (t: number) => void;
    topP: number;
    topPEnabled: boolean;
    onTopPChange: (v: number) => void;
    onTopPEnabledChange: (v: boolean) => void;
    topK: number;
    topKEnabled: boolean;
    onTopKChange: (v: number) => void;
    onTopKEnabledChange: (v: boolean) => void;
    frequencyPenalty: number;
    frequencyPenaltyEnabled: boolean;
    onFrequencyPenaltyChange: (v: number) => void;
    onFrequencyPenaltyEnabledChange: (v: boolean) => void;
    presencePenalty: number;
    presencePenaltyEnabled: boolean;
    onPresencePenaltyChange: (v: number) => void;
    onPresencePenaltyEnabledChange: (v: boolean) => void;
}

export function GenerationSection({
    temperature,
    onTemperatureChange,
    maxTokens,
    onMaxTokensChange,
    topP,
    topPEnabled,
    onTopPChange,
    onTopPEnabledChange,
    topK,
    topKEnabled,
    onTopKChange,
    onTopKEnabledChange,
    frequencyPenalty,
    frequencyPenaltyEnabled,
    onFrequencyPenaltyChange,
    onFrequencyPenaltyEnabledChange,
    presencePenalty,
    presencePenaltyEnabled,
    onPresencePenaltyChange,
    onPresencePenaltyEnabledChange,
}: GenerationSectionProps) {
    const { t } = useTranslation();
    return (
        <Stack gap="xl">
            <Text size="sm" fw={500}>
                {t("settings.generation.title")}
            </Text>
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
                    value={temperature}
                    onChange={onTemperatureChange}
                />
            </Stack>

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("settings.generation.maxTokens")}
                </Text>
                <Text size="xs" c="dimmed">
                    {t("settings.generation.maxTokensDescription")}
                </Text>
                <NumberInput
                    value={maxTokens}
                    onChange={(val) => onMaxTokensChange(Number(val) || 4096)}
                    min={1}
                    max={200000}
                    style={{ maxWidth: 200 }}
                />
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Text size="sm" fw={500}>
                        {t("settings.generation.topP")}
                    </Text>
                    <Checkbox
                        label={t("settings.generation.enable")}
                        checked={topPEnabled}
                        onChange={(e) => onTopPEnabledChange(e.currentTarget.checked)}
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
                        value={topP}
                        onChange={onTopPChange}
                    />
                )}
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Text size="sm" fw={500}>
                        {t("settings.generation.topK")}
                    </Text>
                    <Checkbox
                        label={t("settings.generation.enable")}
                        checked={topKEnabled}
                        onChange={(e) => onTopKEnabledChange(e.currentTarget.checked)}
                    />
                </Group>
                <Text size="xs" c="dimmed">
                    {t("settings.generation.topKDescription")}
                </Text>
                {topKEnabled && (
                    <NumberInput
                        value={topK}
                        onChange={(val) => onTopKChange(Number(val) || 40)}
                        min={1}
                        max={500}
                        style={{ maxWidth: 140 }}
                    />
                )}
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Text size="sm" fw={500}>
                        {t("settings.generation.frequencyPenalty")}
                    </Text>
                    <Checkbox
                        label={t("settings.generation.enable")}
                        checked={frequencyPenaltyEnabled}
                        onChange={(e) =>
                            onFrequencyPenaltyEnabledChange(e.currentTarget.checked)
                        }
                    />
                </Group>
                <Text size="xs" c="dimmed">
                    {t("settings.generation.frequencyPenaltyDescription")}
                </Text>
                {frequencyPenaltyEnabled && (
                    <Slider
                        min={-2}
                        max={2}
                        step={0.1}
                        value={frequencyPenalty}
                        onChange={onFrequencyPenaltyChange}
                    />
                )}
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Text size="sm" fw={500}>
                        {t("settings.generation.presencePenalty")}
                    </Text>
                    <Checkbox
                        label={t("settings.generation.enable")}
                        checked={presencePenaltyEnabled}
                        onChange={(e) =>
                            onPresencePenaltyEnabledChange(e.currentTarget.checked)
                        }
                    />
                </Group>
                <Text size="xs" c="dimmed">
                    {t("settings.generation.presencePenaltyDescription")}
                </Text>
                {presencePenaltyEnabled && (
                    <Slider
                        min={-2}
                        max={2}
                        step={0.1}
                        value={presencePenalty}
                        onChange={onPresencePenaltyChange}
                    />
                )}
            </Stack>
        </Stack>
    );
}

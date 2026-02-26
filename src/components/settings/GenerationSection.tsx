import {
    Box,
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
    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Параметры генерации
                </Text>
                <Group align="flex-end" wrap="nowrap">
                    <Box style={{ flex: 1 }}>
                        <Text size="sm" mb={4}>
                            Температура: {temperature}
                        </Text>
                        <Slider
                            min={0}
                            max={2}
                            step={0.1}
                            value={temperature}
                            onChange={onTemperatureChange}
                        />
                    </Box>
                    <NumberInput
                        label="Макс. токенов"
                        value={maxTokens}
                        onChange={(val) => onMaxTokensChange(Number(val) || 4096)}
                        min={1}
                        max={200000}
                        style={{ width: 140 }}
                    />
                </Group>
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Checkbox
                        label="Top P"
                        checked={topPEnabled}
                        onChange={(e) => onTopPEnabledChange(e.currentTarget.checked)}
                    />
                    {topPEnabled && <Text size="sm">{topP}</Text>}
                </Group>
                <Slider
                    min={0}
                    max={1}
                    step={0.05}
                    value={topP}
                    onChange={onTopPChange}
                    disabled={!topPEnabled}
                />
                <Text size="xs" c="dimmed">
                    Ядровая выборка. Альтернатива температуре — обрезает маловероятные токены.
                    Не рекомендуется использовать вместе с температурой.
                </Text>
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Checkbox
                        label="Top K"
                        checked={topKEnabled}
                        onChange={(e) => onTopKEnabledChange(e.currentTarget.checked)}
                    />
                </Group>
                <NumberInput
                    value={topK}
                    onChange={(val) => onTopKChange(Number(val) || 40)}
                    min={1}
                    max={500}
                    disabled={!topKEnabled}
                    style={{ maxWidth: 140 }}
                />
                <Text size="xs" c="dimmed">
                    Ограничивает выбор из N самых вероятных токенов. Поддерживается не всеми
                    провайдерами.
                </Text>
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Checkbox
                        label="Frequency Penalty"
                        checked={frequencyPenaltyEnabled}
                        onChange={(e) =>
                            onFrequencyPenaltyEnabledChange(e.currentTarget.checked)
                        }
                    />
                    {frequencyPenaltyEnabled && (
                        <Text size="sm">{frequencyPenalty}</Text>
                    )}
                </Group>
                <Slider
                    min={-2}
                    max={2}
                    step={0.1}
                    value={frequencyPenalty}
                    onChange={onFrequencyPenaltyChange}
                    disabled={!frequencyPenaltyEnabled}
                />
                <Text size="xs" c="dimmed">
                    Штраф за частоту. Положительные значения уменьшают повторение слов
                    пропорционально их частоте в тексте.
                </Text>
            </Stack>

            <Stack gap="xs">
                <Group justify="space-between" align="center">
                    <Checkbox
                        label="Presence Penalty"
                        checked={presencePenaltyEnabled}
                        onChange={(e) =>
                            onPresencePenaltyEnabledChange(e.currentTarget.checked)
                        }
                    />
                    {presencePenaltyEnabled && (
                        <Text size="sm">{presencePenalty}</Text>
                    )}
                </Group>
                <Slider
                    min={-2}
                    max={2}
                    step={0.1}
                    value={presencePenalty}
                    onChange={onPresencePenaltyChange}
                    disabled={!presencePenaltyEnabled}
                />
                <Text size="xs" c="dimmed">
                    Штраф за присутствие. Положительные значения уменьшают повторение любых
                    уже использованных слов.
                </Text>
            </Stack>
        </Stack>
    );
}

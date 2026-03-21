import { useTranslation } from "react-i18next";
import {
    Checkbox,
    NumberInput,
    Slider,
    Stack,
    Text,
} from "@mantine/core";
import type { AppSettings } from "../types";
import type { Chat } from "../types";

export interface ChatParamsPopoverProps {
    chat: Chat;
    settings: AppSettings;
    onParamsChange: (
        chatId: string,
        params: {
            temperature?: number | null;
            maxTokens?: number | null;
            topP?: number | null;
            topK?: number | null;
            frequencyPenalty?: number | null;
            presencePenalty?: number | null;
        }
    ) => void;
}

function hasCustomValue(value: number | null | undefined): boolean {
    return value !== undefined && value !== null;
}

export function ChatParamsPopoverContent({
    chat,
    settings,
    onParamsChange,
}: ChatParamsPopoverProps) {
    const { t } = useTranslation();
    const chatId = chat.id;

    const tempCustom = hasCustomValue(chat.temperature);
    const tempVal = tempCustom ? (chat.temperature as number) : (chat.temperature ?? settings.temperature);
    const maxTokCustom = hasCustomValue(chat.maxTokens);
    const maxTokVal = maxTokCustom ? (chat.maxTokens as number) : (chat.maxTokens ?? settings.max_tokens);
    const topPCustom = hasCustomValue(chat.topP);
    const topPVal = topPCustom ? (chat.topP as number) : (chat.topP ?? settings.topP ?? 0.9);
    const topKCustom = hasCustomValue(chat.topK);
    const topKVal = topKCustom ? (chat.topK as number) : (chat.topK ?? settings.topK ?? 40);
    const freqCustom = hasCustomValue(chat.frequencyPenalty);
    const freqVal = freqCustom ? (chat.frequencyPenalty as number) : (chat.frequencyPenalty ?? settings.frequencyPenalty ?? 0);
    const presCustom = hasCustomValue(chat.presencePenalty);
    const presVal = presCustom ? (chat.presencePenalty as number) : (chat.presencePenalty ?? settings.presencePenalty ?? 0);

    const globalTemp = settings.temperature;
    const globalMaxTok = settings.max_tokens;
    const globalTopP = settings.topP ?? "—";
    const globalTopK = settings.topK ?? "—";
    const globalFreq = settings.frequencyPenalty ?? "—";
    const globalPres = settings.presencePenalty ?? "—";

    return (
        <Stack gap="sm">
            <Text size="sm" fw={600}>
                {t("chatParams.title")}
            </Text>

            {/* Temperature */}
            <Stack gap={4}>
                <Checkbox
                    label={t("chatParams.useCustom")}
                    checked={tempCustom}
                    onChange={(e) => {
                        if (e.currentTarget.checked) {
                            onParamsChange(chatId, { temperature: tempVal });
                        } else {
                            onParamsChange(chatId, { temperature: null });
                        }
                    }}
                />
                <Text size="xs" c="dimmed">
                    {t("chatParams.temperature")}
                </Text>
                <Slider
                    min={0}
                    max={2}
                    step={0.1}
                    value={tempVal}
                    onChange={(v) => onParamsChange(chatId, { temperature: v })}
                    disabled={!tempCustom}
                />
                {!tempCustom && (
                    <Text size="xs" c="dimmed">
                        {t("chatParams.globalValue", { value: globalTemp })}
                    </Text>
                )}
            </Stack>

            {/* Max tokens */}
            <Stack gap={4}>
                <Checkbox
                    label={t("chatParams.useCustom")}
                    checked={maxTokCustom}
                    onChange={(e) => {
                        if (e.currentTarget.checked) {
                            onParamsChange(chatId, { maxTokens: maxTokVal });
                        } else {
                            onParamsChange(chatId, { maxTokens: null });
                        }
                    }}
                />
                <NumberInput
                    label={t("chatParams.maxTokens")}
                    value={maxTokVal}
                    onChange={(v) => onParamsChange(chatId, { maxTokens: Number(v) || settings.max_tokens })}
                    min={1}
                    max={200000}
                    disabled={!maxTokCustom}
                    placeholder={!maxTokCustom ? t("chatParams.globalValue", { value: globalMaxTok }) : undefined}
                />
            </Stack>

            {/* Top-P */}
            <Stack gap={4}>
                <Checkbox
                    label={t("chatParams.useCustom")}
                    checked={topPCustom}
                    onChange={(e) => {
                        if (e.currentTarget.checked) {
                            onParamsChange(chatId, { topP: topPVal });
                        } else {
                            onParamsChange(chatId, { topP: null });
                        }
                    }}
                />
                <Text size="xs" c="dimmed">
                    {t("chatParams.topP")}
                </Text>
                <Slider
                    min={0}
                    max={1}
                    step={0.05}
                    value={topPVal}
                    onChange={(v) => onParamsChange(chatId, { topP: v })}
                    disabled={!topPCustom}
                />
                {!topPCustom && (
                    <Text size="xs" c="dimmed">
                        {t("chatParams.globalValue", { value: globalTopP })}
                    </Text>
                )}
            </Stack>

            {/* Top-K */}
            <Stack gap={4}>
                <Checkbox
                    label={t("chatParams.useCustom")}
                    checked={topKCustom}
                    onChange={(e) => {
                        if (e.currentTarget.checked) {
                            onParamsChange(chatId, { topK: topKVal });
                        } else {
                            onParamsChange(chatId, { topK: null });
                        }
                    }}
                />
                <NumberInput
                    label={t("chatParams.topK")}
                    value={topKVal}
                    onChange={(v) => onParamsChange(chatId, { topK: Number(v) ?? (settings.topK ?? 40) })}
                    min={0}
                    max={500}
                    disabled={!topKCustom}
                    placeholder={!topKCustom ? t("chatParams.globalValue", { value: globalTopK }) : undefined}
                />
            </Stack>

            {/* Frequency Penalty */}
            <Stack gap={4}>
                <Checkbox
                    label={t("chatParams.useCustom")}
                    checked={freqCustom}
                    onChange={(e) => {
                        if (e.currentTarget.checked) {
                            onParamsChange(chatId, { frequencyPenalty: freqVal });
                        } else {
                            onParamsChange(chatId, { frequencyPenalty: null });
                        }
                    }}
                />
                <Text size="xs" c="dimmed">
                    {t("chatParams.frequencyPenalty")}
                </Text>
                <Slider
                    min={-2}
                    max={2}
                    step={0.1}
                    value={freqVal}
                    onChange={(v) => onParamsChange(chatId, { frequencyPenalty: v })}
                    disabled={!freqCustom}
                />
                {!freqCustom && (
                    <Text size="xs" c="dimmed">
                        {t("chatParams.globalValue", { value: globalFreq })}
                    </Text>
                )}
            </Stack>

            {/* Presence Penalty */}
            <Stack gap={4}>
                <Checkbox
                    label={t("chatParams.useCustom")}
                    checked={presCustom}
                    onChange={(e) => {
                        if (e.currentTarget.checked) {
                            onParamsChange(chatId, { presencePenalty: presVal });
                        } else {
                            onParamsChange(chatId, { presencePenalty: null });
                        }
                    }}
                />
                <Text size="xs" c="dimmed">
                    {t("chatParams.presencePenalty")}
                </Text>
                <Slider
                    min={-2}
                    max={2}
                    step={0.1}
                    value={presVal}
                    onChange={(v) => onParamsChange(chatId, { presencePenalty: v })}
                    disabled={!presCustom}
                />
                {!presCustom && (
                    <Text size="xs" c="dimmed">
                        {t("chatParams.globalValue", { value: globalPres })}
                    </Text>
                )}
            </Stack>
        </Stack>
    );
}

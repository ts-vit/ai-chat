import { useTranslation } from "react-i18next";
import { Box, Group, Stack, Switch, Text, Title } from "@mantine/core";
import { MODE_DEFINITIONS } from "../../constants/modes";
import { useChatStore } from "../../store/chatStore";
import { notify } from "../../utils/notify";

export function ModesSection() {
    const { t } = useTranslation();
    const { modeSettings, updateModeSetting } = useChatStore();

    const getModeSetting = (modeId: string) =>
        modeSettings.find((s) => s.mode === modeId);

    const handleToggle = async (modeId: string, enabled: boolean) => {
        const enabledCount = modeSettings.filter((s) => s.enabled).length;
        if (!enabled && enabledCount <= 1) {
            notify.warning(t("modes.atLeastOneMode"));
            return;
        }
        const existing = getModeSetting(modeId);
        await updateModeSetting(
            modeId,
            enabled,
            existing?.sortOrder ?? 0,
            existing?.config ?? "{}"
        );
    };

    return (
        <Stack gap="lg">
            <Title order={3}>{t("modes.title")}</Title>
            {MODE_DEFINITIONS.map((mode) => {
                const setting = getModeSetting(mode.id);
                const isEnabled = setting?.enabled ?? mode.defaultEnabled;
                const Icon = mode.icon;

                return (
                    <Box
                        key={mode.id}
                        p="md"
                        style={{
                            border: "1px solid var(--mantine-color-default-border)",
                            borderRadius: "var(--mantine-radius-md)",
                        }}
                    >
                        <Group justify="space-between" wrap="nowrap">
                            <Group gap="sm" wrap="nowrap">
                                <Icon size={24} stroke={1.5} />
                                <Box>
                                    <Text fw={500}>{t(mode.labelKey)}</Text>
                                    <Text size="sm" c="dimmed">
                                        {t(mode.descriptionKey)}
                                    </Text>
                                </Box>
                            </Group>
                            <Switch
                                checked={isEnabled}
                                onChange={(e) =>
                                    handleToggle(mode.id, e.currentTarget.checked)
                                }
                                label={t("modes.enabled")}
                            />
                        </Group>
                        {mode.id === "assistant" && (
                            <Text size="xs" c="dimmed" mt="sm" fs="italic">
                                {t("modes.settingsPlaceholder")}
                            </Text>
                        )}
                    </Box>
                );
            })}
        </Stack>
    );
}

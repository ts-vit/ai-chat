import { useTranslation } from "react-i18next";
import i18n from "../../i18n";
import { Checkbox, Paper, Select, SegmentedControl, Slider, Stack, Switch, Text, useMantineColorScheme } from "@mantine/core";

const STATUS_BAR_METRIC_IDS = ["balance", "context", "tokens", "cost"] as const;

export interface InterfaceSectionProps {
    fontSize: number;
    onFontSizeChange: (size: number) => void;
    language: string;
    onLanguageChange: (value: string) => void;
    sendByEnter: boolean;
    onSendByEnterChange: (value: boolean) => void;
    messageDensity: string;
    onMessageDensityChange: (value: string) => void;
    chatWidth: string;
    onChatWidthChange: (value: string) => void;
    showStatusBar: boolean;
    onShowStatusBarChange: (value: boolean) => void;
    statusBarMetrics: string[];
    onStatusBarMetricsChange: (value: string[]) => void;
}

const LANGUAGE_OPTIONS = [
    { value: "", labelKey: "settings.interface.languageSystem" as const },
    { value: "ru", labelKey: "settings.interface.languageRu" as const },
    { value: "en", labelKey: "settings.interface.languageEn" as const },
];

function getSystemLanguage(): string {
    return navigator.language.startsWith("ru") ? "ru" : "en";
}

export function InterfaceSection({
    fontSize,
    onFontSizeChange,
    language,
    onLanguageChange,
    sendByEnter,
    onSendByEnterChange,
    messageDensity,
    onMessageDensityChange,
    chatWidth,
    onChatWidthChange,
    showStatusBar,
    onShowStatusBarChange,
    statusBarMetrics,
    onStatusBarMetricsChange,
}: InterfaceSectionProps) {
    const { t } = useTranslation();
    const { colorScheme, setColorScheme } = useMantineColorScheme();
    const handleLanguageChange = (value: string | null) => {
        const next = value ?? "";
        onLanguageChange(next);
        i18n.changeLanguage(next || getSystemLanguage());
    };
    const themeValue = colorScheme === "auto" ? "dark" : colorScheme;
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
                    onChange={onMessageDensityChange}
                    data={[
                        { label: t("settings.interface.densityCompact"), value: "compact" },
                        { label: t("settings.interface.densityStandard"), value: "standard" },
                        { label: t("settings.interface.densitySpacious"), value: "spacious" },
                    ]}
                />
            </Stack>
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("settings.interface.chatWidth")}
                </Text>
                <SegmentedControl
                    value={chatWidth}
                    onChange={onChatWidthChange}
                    data={[
                        { label: t("settings.interface.chatWidthNarrow"), value: "narrow" },
                        { label: t("settings.interface.chatWidthStandard"), value: "standard" },
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
                    onChange={(v) => onSendByEnterChange(v === "true")}
                    data={[
                        { label: t("settings.interface.sendByEnter"), value: "true" },
                        { label: t("settings.interface.sendByCtrlEnter"), value: "false" },
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
                    data={LANGUAGE_OPTIONS.map((o) => ({ value: o.value, label: t(o.labelKey) }))}
                    allowDeselect={false}
                />
            </Stack>
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("settings.interface.fontSize")}
                </Text>
                <Text size="sm" mb={4}>
                    {fontSize}px
                </Text>
                <Slider
                    min={12}
                    max={24}
                    step={1}
                    value={fontSize}
                    onChange={onFontSizeChange}
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
                    <Text style={{ fontSize: `${fontSize}px` }}>
                        {t("settings.interface.fontPreview")}
                    </Text>
                </Paper>
            </Stack>
            <Stack gap="xs">
                <Switch
                    label={t("settings.interface.showStatusBar")}
                    checked={showStatusBar}
                    onChange={(e) => onShowStatusBarChange(e.currentTarget.checked)}
                />
                {showStatusBar && (
                    <Checkbox.Group
                        label={t("settings.interface.statusBarMetrics")}
                        value={statusBarMetrics}
                        onChange={onStatusBarMetricsChange}
                    >
                        <Stack gap="xs" mt="xs">
                            {STATUS_BAR_METRIC_IDS.map((id) => (
                                <Checkbox
                                    key={id}
                                    value={id}
                                    label={t(`settings.interface.metric${id.charAt(0).toUpperCase() + id.slice(1)}`)}
                                />
                            ))}
                        </Stack>
                    </Checkbox.Group>
                )}
            </Stack>
        </Stack>
    );
}

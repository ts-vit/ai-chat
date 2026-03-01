import { useTranslation } from "react-i18next";
import i18n from "../../i18n";
import { Select, SegmentedControl, Slider, Stack, Text } from "@mantine/core";

export interface InterfaceSectionProps {
    fontSize: number;
    onFontSizeChange: (size: number) => void;
    language: string;
    onLanguageChange: (value: string) => void;
    sendByEnter: boolean;
    onSendByEnterChange: (value: boolean) => void;
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
}: InterfaceSectionProps) {
    const { t } = useTranslation();
    const handleLanguageChange = (value: string | null) => {
        const next = value ?? "";
        onLanguageChange(next);
        i18n.changeLanguage(next || getSystemLanguage());
    };
    return (
        <Stack gap="lg">
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
                <Text size="xs" c="dimmed">
                    {sendByEnter
                        ? t("settings.interface.sendByEnter")
                        : t("settings.interface.sendByCtrlEnter")}
                </Text>
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
            </Stack>
        </Stack>
    );
}

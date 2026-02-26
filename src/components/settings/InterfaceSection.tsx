import { Slider, Stack, Text } from "@mantine/core";

export interface InterfaceSectionProps {
    fontSize: number;
    onFontSizeChange: (size: number) => void;
}

export function InterfaceSection({ fontSize, onFontSizeChange }: InterfaceSectionProps) {
    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    Размер шрифта сообщений
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

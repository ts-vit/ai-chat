import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Group, Modal, Stack, TextInput } from "@mantine/core";

const VARIABLE_REGEX = /\{([^}]+)\}/g;

function getUniqueVariableNames(content: string): string[] {
    const regex = /\{([^}]+)\}/g;
    const names = new Set<string>();
    let m: RegExpExecArray | null;
    while ((m = regex.exec(content)) !== null) {
        names.add(m[1].trim());
    }
    return Array.from(names);
}

export function fillVariables(content: string, values: Record<string, string>): string {
    return content.replace(VARIABLE_REGEX, (_, name) => {
        const key = name.trim();
        return key in values ? values[key] : `{${name}}`;
    });
}

interface VariablesModalProps {
    content: string;
    opened: boolean;
    onClose: () => void;
    onSubmit: (filledText: string) => void;
}

export function VariablesModal({ content, opened, onClose, onSubmit }: VariablesModalProps) {
    const { t } = useTranslation();
    const variableNames = useMemo(() => getUniqueVariableNames(content), [content]);
    const [values, setValues] = useState<Record<string, string>>({});

    const handleSubmit = () => {
        onSubmit(fillVariables(content, values));
        setValues({});
        onClose();
    };

    return (
        <Modal title={t("variablesModal.title")} size="md" opened={opened} onClose={onClose}>
            <Stack gap="sm">
                {variableNames.map((name) => (
                    <TextInput
                        key={name}
                        label={name}
                        value={values[name] ?? ""}
                        onChange={(e) => {
                            const val = e.currentTarget.value;
                            setValues((prev) => ({ ...prev, [name]: val }));
                        }}
                        placeholder={t("variablesModal.valuePlaceholder", { name })}
                    />
                ))}
                <Group justify="flex-end" gap="sm" mt="md">
                    <Button variant="subtle" onClick={onClose}>
                        {t("variablesModal.cancel")}
                    </Button>
                    <Button onClick={handleSubmit}>{t("variablesModal.insert")}</Button>
                </Group>
            </Stack>
        </Modal>
    );
}

export { getUniqueVariableNames };

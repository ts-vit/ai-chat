import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
    Button,
    Checkbox,
    Group,
    Modal,
    Radio,
    Stack,
    TextInput,
    Textarea,
    Text,
} from "@mantine/core";
import type { TaskTemplate, TaskOptions } from "../constants/taskTemplates";
import { buildPromptFromTemplate } from "../constants/taskTemplates";

interface TaskTemplateModalProps {
    opened: boolean;
    onClose: () => void;
    template: TaskTemplate | null;
    onSubmit: (prompt: string, options: TaskOptions) => void;
}

function getDefaults(template: TaskTemplate): TaskOptions {
    const opts: TaskOptions = {};
    for (const field of template.fields) {
        if (field.defaultValue !== undefined) {
            opts[field.id] = field.defaultValue;
        } else if (field.type === "checkbox") {
            opts[field.id] = false;
        } else {
            opts[field.id] = "";
        }
    }
    return opts;
}

export function TaskTemplateModal({ opened, onClose, template, onSubmit }: TaskTemplateModalProps) {
    const { t } = useTranslation();
    const [options, setOptions] = useState<TaskOptions>({});

    useEffect(() => {
        if (template) {
            setOptions(getDefaults(template));
        }
    }, [template]);

    const setField = useCallback((id: string, value: string | boolean) => {
        setOptions((prev) => ({ ...prev, [id]: value }));
    }, []);

    const canSubmit = template
        ? template.fields
              .filter((f) => f.required)
              .every((f) => {
                  const val = options[f.id];
                  return typeof val === "string" ? val.trim().length > 0 : !!val;
              })
        : false;

    const handleSubmit = useCallback(() => {
        if (!template || !canSubmit) return;
        const prompt = buildPromptFromTemplate(template.id, options);
        onSubmit(prompt, options);
    }, [template, canSubmit, options, onSubmit]);

    if (!template) return null;

    const Icon = template.icon;

    return (
        <Modal
            size="md"
            opened={opened}
            onClose={onClose}
            title={
                <Group gap="xs">
                    <Icon size={20} stroke={1.5} style={{ color: `var(--mantine-color-${template.color}-5)` }} />
                    <Text fw={600}>{t(template.titleKey)}</Text>
                </Group>
            }
        >
            <Stack gap="md">
                {template.fields.map((field) => {
                    switch (field.type) {
                        case "text":
                            return (
                                <TextInput
                                    key={field.id}
                                    label={t(field.label)}
                                    placeholder={field.placeholder ? t(field.placeholder) : undefined}
                                    value={(options[field.id] as string) ?? ""}
                                    onChange={(e) => setField(field.id, e.currentTarget.value)}
                                    required={field.required}
                                    onKeyDown={(e) => {
                                        if (e.key === "Enter" && canSubmit) handleSubmit();
                                    }}
                                    autoFocus={field === template.fields[0]}
                                />
                            );
                        case "textarea":
                            return (
                                <Textarea
                                    key={field.id}
                                    label={t(field.label)}
                                    placeholder={field.placeholder ? t(field.placeholder) : undefined}
                                    value={(options[field.id] as string) ?? ""}
                                    onChange={(e) => setField(field.id, e.currentTarget.value)}
                                    required={field.required}
                                    minRows={2}
                                    maxRows={4}
                                    autosize
                                    autoFocus={field === template.fields[0]}
                                />
                            );
                        case "radio":
                            return (
                                <Radio.Group
                                    key={field.id}
                                    label={t(field.label)}
                                    value={(options[field.id] as string) ?? ""}
                                    onChange={(val) => setField(field.id, val)}
                                >
                                    <Stack gap="xs" mt={4}>
                                        {field.options?.map((opt) => (
                                            <Radio key={opt.value} value={opt.value} label={t(opt.label)} />
                                        ))}
                                    </Stack>
                                </Radio.Group>
                            );
                        case "checkbox":
                            return (
                                <Checkbox
                                    key={field.id}
                                    label={t(field.label)}
                                    checked={!!options[field.id]}
                                    onChange={(e) => setField(field.id, e.currentTarget.checked)}
                                />
                            );
                        default:
                            return null;
                    }
                })}

                <Group justify="flex-end" gap="sm" mt="xs">
                    <Button variant="subtle" onClick={onClose}>
                        {t("common.cancel")}
                    </Button>
                    <Button
                        onClick={handleSubmit}
                        disabled={!canSubmit}
                        color={template.color}
                    >
                        {t("taskTemplate.start")}
                    </Button>
                </Group>
            </Stack>
        </Modal>
    );
}

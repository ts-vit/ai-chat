import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    Button,
    Group,
    Modal,
    Select,
    Stack,
    Text,
    Textarea,
} from "@mantine/core";
import { useChatStore } from "../store/chatStore";

export interface CreateComparisonModalProps {
    opened: boolean;
    onClose: () => void;
    onConfirm: (
        leftProviderId: string,
        leftModel: string,
        rightProviderId: string,
        rightModel: string,
        leftSystemPrompt?: string,
        rightSystemPrompt?: string,
    ) => void;
}

export function CreateComparisonModal({
    opened,
    onClose,
    onConfirm,
}: CreateComparisonModalProps) {
    const { t } = useTranslation();
    const { settings, customProviders, localOllamaModels } = useChatStore();

    const [leftProvider, setLeftProvider] = useState("openrouter");
    const [rightProvider, setRightProvider] = useState("openrouter");
    const [leftModel, setLeftModel] = useState("");
    const [rightModel, setRightModel] = useState("");
    const [leftPrompt, setLeftPrompt] = useState("");
    const [rightPrompt, setRightPrompt] = useState("");

    const providerOptions = useMemo(() => {
        const opts = [
            { value: "openrouter", label: "OpenRouter" },
            { value: "ollama", label: "Ollama" },
        ];
        for (const p of customProviders) {
            opts.push({ value: p.id, label: p.name });
        }
        return opts;
    }, [customProviders]);

    const getModelOptions = useCallback(
        (providerId: string) => {
            if (providerId === "openrouter") {
                return (settings.openrouterEnabledModels ?? []).map((id) => ({ value: id, label: id }));
            }
            if (providerId === "ollama") {
                return localOllamaModels.map((m) => ({ value: m.name, label: m.name }));
            }
            const enabledModels = settings.customProviderEnabledModels?.[providerId] ?? [];
            return enabledModels.map((id) => ({ value: id, label: id }));
        },
        [settings, localOllamaModels]
    );

    const canCreate = leftModel.trim() !== "" && rightModel.trim() !== "";

    return (
        <Modal
            opened={opened}
            onClose={onClose}
            title={t("compare.newComparison")}
            size="lg"
        >
            <Stack gap="md">
                <Group grow align="flex-start">
                    <Stack gap="sm">
                        <Text fw={600} size="sm">{t("compare.leftModel")}</Text>
                        <Select
                            size="xs"
                            label={t("compare.selectLeftProvider")}
                            data={providerOptions}
                            value={leftProvider}
                            onChange={(v) => { setLeftProvider(v ?? "openrouter"); setLeftModel(""); }}
                            allowDeselect={false}
                        />
                        <Select
                            size="xs"
                            label={t("compare.selectLeftModel")}
                            data={getModelOptions(leftProvider)}
                            value={leftModel}
                            onChange={(v) => setLeftModel(v ?? "")}
                            searchable
                            allowDeselect={false}
                        />
                        <Textarea
                            size="xs"
                            label={t("compare.leftSystemPrompt")}
                            value={leftPrompt}
                            onChange={(e) => setLeftPrompt(e.currentTarget.value)}
                            minRows={2}
                            maxRows={4}
                            autosize
                        />
                    </Stack>
                    <Stack gap="sm">
                        <Text fw={600} size="sm">{t("compare.rightModel")}</Text>
                        <Select
                            size="xs"
                            label={t("compare.selectRightProvider")}
                            data={providerOptions}
                            value={rightProvider}
                            onChange={(v) => { setRightProvider(v ?? "openrouter"); setRightModel(""); }}
                            allowDeselect={false}
                        />
                        <Select
                            size="xs"
                            label={t("compare.selectRightModel")}
                            data={getModelOptions(rightProvider)}
                            value={rightModel}
                            onChange={(v) => setRightModel(v ?? "")}
                            searchable
                            allowDeselect={false}
                        />
                        <Textarea
                            size="xs"
                            label={t("compare.rightSystemPrompt")}
                            value={rightPrompt}
                            onChange={(e) => setRightPrompt(e.currentTarget.value)}
                            minRows={2}
                            maxRows={4}
                            autosize
                        />
                    </Stack>
                </Group>
                <Group justify="flex-end" gap="sm">
                    <Button variant="subtle" onClick={onClose}>
                        {t("common.cancel")}
                    </Button>
                    <Button
                        disabled={!canCreate}
                        onClick={() => {
                            onConfirm(
                                leftProvider,
                                leftModel,
                                rightProvider,
                                rightModel,
                                leftPrompt.trim() || undefined,
                                rightPrompt.trim() || undefined,
                            );
                            onClose();
                        }}
                    >
                        {t("common.create")}
                    </Button>
                </Group>
            </Stack>
        </Modal>
    );
}

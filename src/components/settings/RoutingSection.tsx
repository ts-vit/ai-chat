import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Box, Button, SegmentedControl, Select, Stack, Switch, Table, Text, Title } from "@mantine/core";
import { IconRoute } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "../../utils/notify";
import { ModelCatalogSection } from "./ModelCatalogSection";
import type { ModelCatalogEntry, RoutingRule } from "../../types";

interface RoutingSectionProps {
    routingEnabled: boolean;
    onRoutingEnabledChange: (v: boolean) => void;
    routingStrategy: "rules" | "llm";
    onRoutingStrategyChange: (v: "rules" | "llm") => void;
    onSyncModels: () => void;
    lastSync?: number | null;
}

const CATEGORIES = ["coding", "writing", "analysis", "general", "fast", "vision"] as const;

export function RoutingSection({
    routingEnabled,
    onRoutingEnabledChange,
    routingStrategy,
    onRoutingStrategyChange,
    onSyncModels,
    lastSync,
}: RoutingSectionProps) {
    const { t } = useTranslation();
    const [rules, setRules] = useState<RoutingRule[]>([]);
    const [catalog, setCatalog] = useState<ModelCatalogEntry[]>([]);
    const [rulesLoading, setRulesLoading] = useState(false);

    const loadRules = useCallback(async () => {
        setRulesLoading(true);
        try {
            const list = await invoke<RoutingRule[]>("get_routing_rules");
            setRules(list ?? []);
        } catch (e) {
            notify.error(String(e));
        } finally {
            setRulesLoading(false);
        }
    }, []);

    const loadCatalog = useCallback(async () => {
        try {
            const list = await invoke<ModelCatalogEntry[]>("get_model_catalog");
            setCatalog(list ?? []);
        } catch {
            setCatalog([]);
        }
    }, []);

    useEffect(() => {
        if (routingStrategy === "rules" && routingEnabled) {
            loadRules();
            loadCatalog();
        }
    }, [routingStrategy, routingEnabled, loadRules, loadCatalog]);

    const handleSync = async () => {
        try {
            await invoke("sync_model_catalog");
            notify.success(t("routing.syncModels") + " — OK");
            onSyncModels();
            if (routingStrategy === "rules") {
                loadCatalog();
            }
        } catch (e) {
            notify.error(String(e));
        }
    };

    const updateRule = useCallback(
        async (id: number, patch: { preferredModel?: string; fallbackModel?: string; enabled?: boolean }) => {
            try {
                await invoke("update_routing_rule", {
                    id,
                    preferredModel: patch.preferredModel ?? undefined,
                    fallbackModel: patch.fallbackModel !== undefined ? patch.fallbackModel ?? "" : undefined,
                    enabled: patch.enabled ?? undefined,
                });
                                setRules((prev) =>
                                    prev.map((r) =>
                                        r.id === id
                                            ? {
                                                  ...r,
                                                  preferredModel: patch.preferredModel ?? r.preferredModel,
                                                  fallbackModel:
                                                      patch.fallbackModel !== undefined ? patch.fallbackModel : r.fallbackModel,
                                                  enabled: patch.enabled ?? r.enabled,
                                              }
                                            : r
                                    )
                                );
            } catch (e) {
                notify.error(String(e));
            }
        },
        []
    );

    const handleResetRules = useCallback(async () => {
        try {
            await invoke("reset_routing_rules_to_defaults");
            await loadRules();
            notify.success(t("routing.resetDefaults"));
        } catch (e) {
            notify.error(String(e));
        }
    }, [loadRules, t]);

    const availableModels = catalog.filter((e) => e.isAvailable);
    const modelOptions = availableModels.map((e) => ({ value: e.id, label: e.displayName }));

    return (
        <Stack gap="lg">
            <Title order={3}>{t("routing.title")}</Title>
            <Box
                p="md"
                style={{
                    border: "1px solid var(--mantine-color-default-border)",
                    borderRadius: "var(--mantine-radius-md)",
                }}
            >
                <Stack gap="md">
                    <SegmentedControl
                        value={routingEnabled ? "on" : "off"}
                        onChange={(v: string) => onRoutingEnabledChange(v === "on")}
                        data={[
                            { label: t("routing.enabled"), value: "on" },
                            { label: "Off", value: "off" },
                        ]}
                    />
                    {routingEnabled && (
                        <>
                            <Text size="sm" fw={500}>
                                {t("routing.strategy")}
                            </Text>
                            <SegmentedControl
                                value={routingStrategy}
                                onChange={(v: string) => onRoutingStrategyChange(v as "rules" | "llm")}
                                data={[
                                    { label: t("routing.strategyRules"), value: "rules" },
                                    { label: t("routing.strategyLlm"), value: "llm" },
                                ]}
                            />
                            {routingStrategy === "rules" && (
                                <>
                                    <Text size="sm" c="dimmed">
                                        {t("routing.rulesDescription")}
                                    </Text>
                                    <Box mt="md">
                                        <Button variant="subtle" size="xs" mb="sm" onClick={handleResetRules}>
                                            {t("routing.resetRules")}
                                        </Button>
                                        {rulesLoading ? (
                                            <Text size="sm" c="dimmed">
                                                {t("common.loading")}
                                            </Text>
                                        ) : (
                                            <Table striped="even" withTableBorder withColumnBorders>
                                                <Table.Thead>
                                                    <Table.Tr>
                                                        <Table.Th>{t("routing.category")}</Table.Th>
                                                        <Table.Th>{t("routing.preferredModel")}</Table.Th>
                                                        <Table.Th>{t("routing.fallbackModel")}</Table.Th>
                                                        <Table.Th>{t("routing.enabled")}</Table.Th>
                                                    </Table.Tr>
                                                </Table.Thead>
                                                <Table.Tbody>
                                                    {rules.map((rule) => (
                                                        <Table.Tr key={rule.id}>
                                                            <Table.Td>
                                                                <Text size="sm">
                                                                    {CATEGORIES.includes(rule.taskCategory as (typeof CATEGORIES)[number])
                                                                        ? t(`routing.${rule.taskCategory}`)
                                                                        : rule.taskCategory}
                                                                </Text>
                                                            </Table.Td>
                                                            <Table.Td>
                                                                <Select
                                                                    size="xs"
                                                                    data={modelOptions}
                                                                    value={rule.preferredModel || ""}
                                                                    onChange={(v) =>
                                                                        updateRule(rule.id, { preferredModel: v ?? "" })
                                                                    }
                                                                    placeholder="—"
                                                                    clearable
                                                                    styles={{ input: { minHeight: 28 } }}
                                                                />
                                                            </Table.Td>
                                                            <Table.Td>
                                                                <Select
                                                                    size="xs"
                                                                    data={[
                                                                        { value: "", label: t("routing.default") },
                                                                        ...modelOptions,
                                                                    ]}
                                                                    value={
                                                                        rule.fallbackModel == null || rule.fallbackModel === ""
                                                                            ? ""
                                                                            : rule.fallbackModel
                                                                    }
                                                                    onChange={(v) =>
                                                                        updateRule(rule.id, {
                                                                            fallbackModel: v ?? "",
                                                                        })
                                                                    }
                                                                    styles={{ input: { minHeight: 28 } }}
                                                                />
                                                            </Table.Td>
                                                            <Table.Td>
                                                                <Switch
                                                                    size="sm"
                                                                    checked={rule.enabled}
                                                                    onChange={(e) =>
                                                                        updateRule(rule.id, {
                                                                            enabled: e.currentTarget.checked,
                                                                        })
                                                                    }
                                                                />
                                                            </Table.Td>
                                                        </Table.Tr>
                                                    ))}
                                                </Table.Tbody>
                                            </Table>
                                        )}
                                    </Box>
                                </>
                            )}
                            {routingStrategy === "llm" && (
                                <Text size="sm" c="dimmed">
                                    {t("routing.llmDescription")}
                                </Text>
                            )}
                        </>
                    )}
                </Stack>
            </Box>
            <Box>
                <Text size="sm" fw={500} mb="xs">
                    {t("routing.modelCatalog")}
                </Text>
                {lastSync != null && (
                    <Text size="xs" c="dimmed" mb="xs">
                        {t("routing.lastSync")}: {new Date(lastSync * 1000).toLocaleString()}
                    </Text>
                )}
                <Button variant="light" size="sm" leftSection={<IconRoute size={16} stroke={1.5} />} onClick={handleSync}>
                    {t("routing.syncModels")}
                </Button>
                <Box mt="md">
                    <ModelCatalogSection lastSync={lastSync ?? undefined} onCatalogUpdated={onSyncModels} />
                </Box>
            </Box>
        </Stack>
    );
}

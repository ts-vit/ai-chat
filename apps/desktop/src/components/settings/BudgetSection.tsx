import { useTranslation } from "react-i18next";
import { Box, NumberInput, Select, Stack, Switch, Title } from "@mantine/core";

interface BudgetSectionProps {
    budgetPlanEnabled: boolean;
    onBudgetPlanEnabledChange: (v: boolean) => void;
    budgetPlanLimit: number;
    onBudgetPlanLimitChange: (v: number) => void;
    budgetGlobalEnabled: boolean;
    onBudgetGlobalEnabledChange: (v: boolean) => void;
    budgetGlobalLimit: number;
    onBudgetGlobalLimitChange: (v: number) => void;
    budgetGlobalPeriod: "daily" | "monthly";
    onBudgetGlobalPeriodChange: (v: "daily" | "monthly") => void;
}

export function BudgetSection({
    budgetPlanEnabled,
    onBudgetPlanEnabledChange,
    budgetPlanLimit,
    onBudgetPlanLimitChange,
    budgetGlobalEnabled,
    onBudgetGlobalEnabledChange,
    budgetGlobalLimit,
    onBudgetGlobalLimitChange,
    budgetGlobalPeriod,
    onBudgetGlobalPeriodChange,
}: BudgetSectionProps) {
    const { t } = useTranslation();

    return (
        <Stack gap="lg">
            <Title order={3}>{t("budget.title")}</Title>
            <Box
                p="md"
                style={{
                    border: "1px solid var(--mantine-color-default-border)",
                    borderRadius: "var(--mantine-radius-md)",
                }}
            >
                <Stack gap="md">
                    <Switch
                        label={t("budget.perPlan")}
                        checked={budgetPlanEnabled}
                        onChange={(e) => onBudgetPlanEnabledChange(e.currentTarget.checked)}
                    />
                    {budgetPlanEnabled && (
                        <NumberInput
                            label={t("budget.limit")}
                            value={budgetPlanLimit || ""}
                            onChange={(v) => onBudgetPlanLimitChange(Number(v) || 0)}
                            min={0}
                            step={0.5}
                            decimalScale={2}
                            suffix=" USD"
                        />
                    )}
                </Stack>
            </Box>
            <Box
                p="md"
                style={{
                    border: "1px solid var(--mantine-color-default-border)",
                    borderRadius: "var(--mantine-radius-md)",
                }}
            >
                <Stack gap="md">
                    <Switch
                        label={t("budget.global")}
                        checked={budgetGlobalEnabled}
                        onChange={(e) => onBudgetGlobalEnabledChange(e.currentTarget.checked)}
                    />
                    {budgetGlobalEnabled && (
                        <>
                            <Select
                                label={t("budget.period")}
                                value={budgetGlobalPeriod}
                                onChange={(v) => onBudgetGlobalPeriodChange((v as "daily" | "monthly") || "daily")}
                                data={[
                                    { value: "daily", label: t("budget.daily") },
                                    { value: "monthly", label: t("budget.monthly") },
                                ]}
                            />
                            <NumberInput
                                label={t("budget.limit")}
                                value={budgetGlobalLimit || ""}
                                onChange={(v) => onBudgetGlobalLimitChange(Number(v) || 0)}
                                min={0}
                                step={0.5}
                                decimalScale={2}
                                suffix=" USD"
                            />
                        </>
                    )}
                </Stack>
            </Box>
        </Stack>
    );
}

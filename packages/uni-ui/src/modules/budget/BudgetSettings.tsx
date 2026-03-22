import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  Box,
  Loader,
  NumberInput,
  Select,
  Stack,
  Switch,
  Title,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useSettings } from "../../settings/useSettings";

export function BudgetSettings() {
  const { t } = useTranslation();

  const planEnabledSetting = useSettings("budget.plan.enabled");
  const planLimitSetting = useSettings("budget.plan.limit");
  const globalEnabledSetting = useSettings("budget.global.enabled");
  const globalLimitSetting = useSettings("budget.global.limit");
  const globalPeriodSetting = useSettings("budget.global.period");

  const planEnabled = planEnabledSetting.value === "true";
  const globalEnabled = globalEnabledSetting.value === "true";
  const globalPeriod = (globalPeriodSetting.value as "daily" | "monthly") ?? "daily";

  const [planLimitLocal, setPlanLimitLocal] = useState<number | string>(5);
  const [globalLimitLocal, setGlobalLimitLocal] = useState<number | string>(10);

  useEffect(() => {
    setPlanLimitLocal(parseFloat(planLimitSetting.value ?? "5") || 5);
  }, [planLimitSetting.value]);

  useEffect(() => {
    setGlobalLimitLocal(parseFloat(globalLimitSetting.value ?? "10") || 10);
  }, [globalLimitSetting.value]);

  const prevRef = useRef<Record<string, string>>({});

  const showSaved = useCallback(
    (key: string, value: string) => {
      if (prevRef.current[key] === value) return;
      prevRef.current[key] = value;
      notifications.show({
        message: t("budget.saved"),
        color: "green",
        autoClose: 2000,
      });
    },
    [t],
  );

  const isLoading =
    planEnabledSetting.loading ||
    planLimitSetting.loading ||
    globalEnabledSetting.loading ||
    globalLimitSetting.loading ||
    globalPeriodSetting.loading;

  if (isLoading) {
    return <Loader size="sm" />;
  }

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
            checked={planEnabled}
            onChange={(e) => {
              const v = e.currentTarget.checked ? "true" : "false";
              planEnabledSetting.set(v);
              showSaved("planEnabled", v);
            }}
          />
          {planEnabled && (
            <NumberInput
              label={t("budget.limit")}
              value={planLimitLocal}
              onChange={(v) => setPlanLimitLocal(v)}
              onBlur={() => {
                const num = Number(planLimitLocal) || 5;
                const s = String(num);
                planLimitSetting.set(s);
                showSaved("planLimit", s);
              }}
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
            checked={globalEnabled}
            onChange={(e) => {
              const v = e.currentTarget.checked ? "true" : "false";
              globalEnabledSetting.set(v);
              showSaved("globalEnabled", v);
            }}
          />
          {globalEnabled && (
            <>
              <Select
                label={t("budget.period")}
                value={globalPeriod}
                onChange={(v) => {
                  const val = (v as "daily" | "monthly") || "daily";
                  globalPeriodSetting.set(val);
                  showSaved("globalPeriod", val);
                }}
                data={[
                  { value: "daily", label: t("budget.daily") },
                  { value: "monthly", label: t("budget.monthly") },
                ]}
              />
              <NumberInput
                label={t("budget.limit")}
                value={globalLimitLocal}
                onChange={(v) => setGlobalLimitLocal(v)}
                onBlur={() => {
                  const num = Number(globalLimitLocal) || 10;
                  const s = String(num);
                  globalLimitSetting.set(s);
                  showSaved("globalLimit", s);
                }}
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

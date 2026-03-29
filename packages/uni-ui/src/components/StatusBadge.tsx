import { Badge, type MantineSize } from "@mantine/core";

export type StatusVariant =
  | "connected"
  | "disconnected"
  | "running"
  | "idle"
  | "error"
  | "warning"
  | "unknown"
  | "auth_required";

export interface StatusBadgeProps {
  /** Status variant — determines color */
  status: StatusVariant;
  /** Custom label. If not provided, uses capitalized status name */
  label?: string;
  /** Badge size. Default: "sm" */
  size?: MantineSize;
  /** Whether to show a pulsing dot before the label. Default: auto (true for running/connected) */
  dot?: boolean;
  /** Custom color override (bypasses status-based color) */
  color?: string;
  /** Additional labels map for i18n — maps status to display text */
  labels?: Partial<Record<StatusVariant, string>>;
}

const STATUS_COLORS: Record<StatusVariant, string> = {
  connected: "green",
  disconnected: "gray",
  running: "green",
  idle: "gray",
  error: "red",
  warning: "yellow",
  unknown: "gray",
  auth_required: "yellow",
};

const DEFAULT_LABELS: Record<StatusVariant, string> = {
  connected: "Connected",
  disconnected: "Disconnected",
  running: "Running",
  idle: "Idle",
  error: "Error",
  warning: "Warning",
  unknown: "Unknown",
  auth_required: "Auth Required",
};

const DOT_STATUSES: Set<StatusVariant> = new Set(["connected", "running"]);

export function StatusBadge({
  status,
  label,
  size = "sm",
  dot,
  color,
  labels,
}: StatusBadgeProps) {
  const badgeColor = color ?? STATUS_COLORS[status] ?? "gray";
  const displayLabel =
    label ?? labels?.[status] ?? DEFAULT_LABELS[status] ?? status;
  const showDot = dot ?? DOT_STATUSES.has(status);

  return (
    <Badge
      color={badgeColor}
      variant="light"
      size={size}
      leftSection={
        showDot ? (
          <div
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              backgroundColor: `var(--mantine-color-${badgeColor}-5)`,
              animation:
                status === "running"
                  ? "uni-pulse 1.5s ease-in-out infinite"
                  : undefined,
            }}
          />
        ) : undefined
      }
      styles={{
        root: {
          textTransform: "uppercase" as const,
          letterSpacing: 0.5,
        },
      }}
    >
      {displayLabel}
      {status === "running" && (
        <style>{`
          @keyframes uni-pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.4; }
          }
        `}</style>
      )}
    </Badge>
  );
}

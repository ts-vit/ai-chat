import { ReactNode } from "react";
import { Stack, Text, type MantineSize } from "@mantine/core";

export interface EmptyStateProps {
  /** Icon element to display */
  icon?: ReactNode;
  /** Main title/message */
  title: string;
  /** Optional description below title */
  description?: string;
  /** Optional action element (button, link) below description */
  action?: ReactNode;
  /** Vertical padding. Default: "xl" */
  py?: MantineSize | number;
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  py = "xl",
}: EmptyStateProps) {
  return (
    <Stack align="center" justify="center" gap="md" py={py} style={{ flex: 1 }}>
      {icon}
      <Text fw={500} c="dimmed" size="sm">
        {title}
      </Text>
      {description && (
        <Text c="dimmed" size="xs">
          {description}
        </Text>
      )}
      {action}
    </Stack>
  );
}

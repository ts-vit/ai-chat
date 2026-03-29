import { useState } from "react";
import {
  ActionIcon,
  CloseButton,
  Group,
  ScrollArea,
  Text,
  Tooltip,
  UnstyledButton,
} from "@mantine/core";
import { IconPlus } from "@tabler/icons-react";

/** Status of a session tab */
export type TabStatus = "idle" | "running" | "error";

/** A single tab definition */
export interface SessionTab {
  /** Unique identifier */
  id: string;
  /** Display label */
  label: string;
  /** Current status */
  status?: TabStatus;
  /** Whether this tab can be closed (default: true) */
  closable?: boolean;
}

/** Props for SessionTabs component */
export interface SessionTabsProps {
  /** List of tabs to display */
  tabs: SessionTab[];
  /** ID of the currently active tab */
  activeId: string;
  /** Called when a tab is clicked */
  onTabChange: (tabId: string) => void;
  /** Called when the add button is clicked. If not provided, add button is hidden. */
  onTabAdd?: () => void;
  /** Called when a tab's close button is clicked */
  onTabClose?: (tabId: string) => void;
  /** Maximum number of tabs allowed (add button disabled when reached). Default: no limit */
  maxTabs?: number;
  /** Height of the tab bar in px. Default: 32 */
  height?: number;
  /** Tooltip for add button. Default: "New tab" */
  addTooltip?: string;
  /** Tooltip for close button. Default: "Close tab" */
  closeTooltip?: string;
}

export function SessionTabs({
  tabs,
  activeId,
  onTabChange,
  onTabAdd,
  onTabClose,
  maxTabs,
  height = 32,
  addTooltip = "New tab",
  closeTooltip = "Close tab",
}: SessionTabsProps) {
  return (
    <Group
      gap={0}
      wrap="nowrap"
      style={{
        height,
        minHeight: height,
        borderBottom: "1px solid var(--mantine-color-default-border)",
        overflow: "hidden",
      }}
    >
      <ScrollArea type="never" style={{ flex: 1 }} scrollbarSize={0}>
        <Group gap={0} wrap="nowrap">
          {tabs.map((tab) => (
            <TabItem
              key={tab.id}
              tab={tab}
              isActive={tab.id === activeId}
              onClick={() => onTabChange(tab.id)}
              onClose={onTabClose ? () => onTabClose(tab.id) : undefined}
              closeTooltip={closeTooltip}
            />
          ))}
        </Group>
      </ScrollArea>

      {onTabAdd && (
        <Tooltip label={addTooltip} position="bottom">
          <ActionIcon
            variant="subtle"
            size="sm"
            onClick={onTabAdd}
            disabled={maxTabs !== undefined && tabs.length >= maxTabs}
            aria-label={addTooltip}
            style={{
              flexShrink: 0,
              marginRight: 4,
              marginLeft: 4,
            }}
          >
            <IconPlus size={14} stroke={1.5} />
          </ActionIcon>
        </Tooltip>
      )}
    </Group>
  );
}

interface TabItemProps {
  tab: SessionTab;
  isActive: boolean;
  onClick: () => void;
  onClose?: () => void;
  closeTooltip: string;
}

function TabItem({ tab, isActive, onClick, onClose, closeTooltip }: TabItemProps) {
  const [hovered, setHovered] = useState(false);
  const closable = tab.closable !== false;
  const showClose = closable && onClose && (hovered || isActive);

  return (
    <UnstyledButton
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 6,
        padding: "0 12px",
        height: "100%",
        borderRight: "1px solid var(--mantine-color-default-border)",
        backgroundColor: isActive
          ? "var(--mantine-color-dark-5)"
          : hovered
            ? "var(--mantine-color-dark-6)"
            : "transparent",
        cursor: "pointer",
        whiteSpace: "nowrap",
        position: "relative",
        borderBottom: isActive
          ? "2px solid var(--mantine-color-brand-5)"
          : "2px solid transparent",
        transition: "background-color 0.1s",
      }}
    >
      {tab.status && tab.status !== "idle" && (
        <div
          data-testid={`status-${tab.id}`}
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            backgroundColor:
              tab.status === "running"
                ? "var(--mantine-color-green-5)"
                : "var(--mantine-color-red-5)",
            flexShrink: 0,
          }}
        />
      )}

      <Text
        size="xs"
        fw={isActive ? 600 : 400}
        c={isActive ? undefined : "dimmed"}
        style={{ userSelect: "none" }}
      >
        {tab.label}
      </Text>

      {showClose ? (
        <Tooltip label={closeTooltip} position="bottom">
          <CloseButton
            size="xs"
            variant="subtle"
            onClick={(e) => {
              e.stopPropagation();
              onClose?.();
            }}
            style={{ flexShrink: 0 }}
          />
        </Tooltip>
      ) : closable && onClose ? (
        <div style={{ width: 18, flexShrink: 0 }} />
      ) : null}
    </UnstyledButton>
  );
}

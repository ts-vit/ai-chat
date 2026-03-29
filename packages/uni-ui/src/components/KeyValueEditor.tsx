import { ActionIcon, Button, Group, Stack, TextInput, Text } from "@mantine/core";
import { IconPlus, IconX } from "@tabler/icons-react";

export interface KeyValuePair {
  key: string;
  value: string;
}

export interface KeyValueEditorProps {
  /** Current list of key-value pairs */
  value: KeyValuePair[];
  /** Called when the list changes */
  onChange: (pairs: KeyValuePair[]) => void;
  /** Label above the editor */
  label?: string;
  /** Placeholder for key input */
  keyPlaceholder?: string;
  /** Placeholder for value input */
  valuePlaceholder?: string;
  /** Label for the add button. Default: "Add" */
  addLabel?: string;
  /** Maximum number of pairs. Default: no limit */
  maxPairs?: number;
  /** Whether the editor is disabled */
  disabled?: boolean;
}

export function KeyValueEditor({
  value,
  onChange,
  label,
  keyPlaceholder = "Key",
  valuePlaceholder = "Value",
  addLabel = "Add",
  maxPairs,
  disabled = false,
}: KeyValueEditorProps) {
  const handleKeyChange = (index: number, newKey: string) => {
    const updated = [...value];
    updated[index] = { ...updated[index], key: newKey };
    onChange(updated);
  };

  const handleValueChange = (index: number, newValue: string) => {
    const updated = [...value];
    updated[index] = { ...updated[index], value: newValue };
    onChange(updated);
  };

  const handleAdd = () => {
    onChange([...value, { key: "", value: "" }]);
  };

  const handleRemove = (index: number) => {
    const updated = value.filter((_, i) => i !== index);
    onChange(updated);
  };

  const canAdd = !disabled && (maxPairs === undefined || value.length < maxPairs);

  return (
    <Stack gap="xs">
      {label && (
        <Text size="sm" fw={500}>
          {label}
        </Text>
      )}

      {value.map((pair, index) => (
        <Group key={index} gap="xs" wrap="nowrap">
          <TextInput
            placeholder={keyPlaceholder}
            value={pair.key}
            onChange={(e) => handleKeyChange(index, e.currentTarget.value)}
            disabled={disabled}
            size="xs"
            style={{ flex: 1 }}
          />
          <TextInput
            placeholder={valuePlaceholder}
            value={pair.value}
            onChange={(e) => handleValueChange(index, e.currentTarget.value)}
            disabled={disabled}
            size="xs"
            style={{ flex: 1 }}
          />
          <ActionIcon
            variant="subtle"
            color="red"
            size="sm"
            onClick={() => handleRemove(index)}
            disabled={disabled}
          >
            <IconX size={14} stroke={1.5} />
          </ActionIcon>
        </Group>
      ))}

      <Button
        variant="subtle"
        size="xs"
        leftSection={<IconPlus size={14} stroke={1.5} />}
        onClick={handleAdd}
        disabled={!canAdd}
        style={{ alignSelf: "flex-start" }}
      >
        {addLabel}
      </Button>
    </Stack>
  );
}

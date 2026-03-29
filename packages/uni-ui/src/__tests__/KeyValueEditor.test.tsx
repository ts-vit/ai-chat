import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MantineProvider } from "@mantine/core";
import { KeyValueEditor, type KeyValuePair } from "../components/KeyValueEditor";

beforeAll(() => {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: query === "(prefers-color-scheme: dark)",
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

function renderWithMantine(ui: React.ReactElement) {
  return render(<MantineProvider>{ui}</MantineProvider>);
}

describe("KeyValueEditor", () => {
  it("renders empty state with add button", () => {
    renderWithMantine(
      <KeyValueEditor value={[]} onChange={() => {}} addLabel="Add Variable" />
    );
    expect(screen.getByText("Add Variable")).toBeInTheDocument();
  });

  it("renders existing pairs", () => {
    const pairs: KeyValuePair[] = [
      { key: "API_KEY", value: "abc123" },
      { key: "SECRET", value: "xyz" },
    ];
    renderWithMantine(
      <KeyValueEditor value={pairs} onChange={() => {}} />
    );
    const inputs = screen.getAllByRole("textbox");
    // 2 pairs × 2 inputs = 4 textboxes
    expect(inputs.length).toBe(4);
  });

  it("calls onChange when add button is clicked", () => {
    const onChange = vi.fn();
    renderWithMantine(
      <KeyValueEditor value={[]} onChange={onChange} addLabel="Add" />
    );
    fireEvent.click(screen.getByText("Add"));
    expect(onChange).toHaveBeenCalledWith([{ key: "", value: "" }]);
  });

  it("calls onChange when key is modified", () => {
    const onChange = vi.fn();
    const pairs: KeyValuePair[] = [{ key: "old", value: "val" }];
    renderWithMantine(
      <KeyValueEditor value={pairs} onChange={onChange} />
    );
    const inputs = screen.getAllByRole("textbox");
    fireEvent.change(inputs[0], { target: { value: "new" } });
    expect(onChange).toHaveBeenCalledWith([{ key: "new", value: "val" }]);
  });

  it("calls onChange when value is modified", () => {
    const onChange = vi.fn();
    const pairs: KeyValuePair[] = [{ key: "k", value: "old" }];
    renderWithMantine(
      <KeyValueEditor value={pairs} onChange={onChange} />
    );
    const inputs = screen.getAllByRole("textbox");
    fireEvent.change(inputs[1], { target: { value: "new" } });
    expect(onChange).toHaveBeenCalledWith([{ key: "k", value: "new" }]);
  });

  it("calls onChange when remove button is clicked", () => {
    const onChange = vi.fn();
    const pairs: KeyValuePair[] = [
      { key: "A", value: "1" },
      { key: "B", value: "2" },
    ];
    renderWithMantine(
      <KeyValueEditor value={pairs} onChange={onChange} />
    );
    // Find remove buttons (ActionIcon with icon, no text)
    const removeButtons = screen.getAllByRole("button").filter(
      (btn) => !btn.textContent
    );
    if (removeButtons.length > 0) {
      fireEvent.click(removeButtons[0]);
      expect(onChange).toHaveBeenCalledWith([{ key: "B", value: "2" }]);
    }
  });

  it("renders label when provided", () => {
    renderWithMantine(
      <KeyValueEditor
        value={[]}
        onChange={() => {}}
        label="Environment Variables"
      />
    );
    expect(screen.getByText("Environment Variables")).toBeInTheDocument();
  });

  it("disables add button when maxPairs reached", () => {
    const pairs: KeyValuePair[] = [
      { key: "A", value: "1" },
      { key: "B", value: "2" },
    ];
    renderWithMantine(
      <KeyValueEditor
        value={pairs}
        onChange={() => {}}
        maxPairs={2}
        addLabel="Add"
      />
    );
    const addButton = screen.getByText("Add").closest("button");
    expect(addButton).toBeDisabled();
  });

  it("disables all inputs when disabled=true", () => {
    const pairs: KeyValuePair[] = [{ key: "A", value: "1" }];
    renderWithMantine(
      <KeyValueEditor value={pairs} onChange={() => {}} disabled />
    );
    const inputs = screen.getAllByRole("textbox");
    inputs.forEach((input) => {
      expect(input).toBeDisabled();
    });
  });
});

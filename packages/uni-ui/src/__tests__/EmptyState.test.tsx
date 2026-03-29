import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen } from "@testing-library/react";
import { MantineProvider } from "@mantine/core";
import { EmptyState } from "../components/EmptyState";

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

describe("EmptyState", () => {
  it("renders title", () => {
    renderWithMantine(<EmptyState title="No items found" />);
    expect(screen.getByText("No items found")).toBeInTheDocument();
  });

  it("renders description when provided", () => {
    renderWithMantine(
      <EmptyState title="Empty" description="Add something to get started" />
    );
    expect(screen.getByText("Add something to get started")).toBeInTheDocument();
  });

  it("does not render description when not provided", () => {
    renderWithMantine(<EmptyState title="Empty" />);
    const texts = screen.getAllByText(/./);
    expect(texts.length).toBe(1);
  });

  it("renders icon when provided", () => {
    renderWithMantine(
      <EmptyState title="Empty" icon={<span data-testid="icon">🔍</span>} />
    );
    expect(screen.getByTestId("icon")).toBeInTheDocument();
  });

  it("renders action when provided", () => {
    renderWithMantine(
      <EmptyState title="Empty" action={<button>Add Item</button>} />
    );
    expect(screen.getByText("Add Item")).toBeInTheDocument();
  });

  it("renders all props together", () => {
    renderWithMantine(
      <EmptyState
        title="No results"
        description="Try a different search"
        icon={<span data-testid="search-icon">🔍</span>}
        action={<button>Clear filters</button>}
      />
    );
    expect(screen.getByText("No results")).toBeInTheDocument();
    expect(screen.getByText("Try a different search")).toBeInTheDocument();
    expect(screen.getByTestId("search-icon")).toBeInTheDocument();
    expect(screen.getByText("Clear filters")).toBeInTheDocument();
  });
});

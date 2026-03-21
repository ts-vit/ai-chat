import { createTheme } from "@mantine/core";
import { brandOrange } from "./brandPalette";

/**
 * UNI default theme — brand orange, Inter + JetBrains Mono, component overrides.
 */
export const uniTheme = createTheme({
  primaryColor: "brand",
  colors: {
    brand: brandOrange,
  },
  fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
  fontFamilyMonospace: "'JetBrains Mono', 'Fira Code', monospace",
  components: {
    Divider: {
      defaultProps: {
        color: "brand.2",
      },
    },
    NavLink: {
      defaultProps: {
        color: "brand",
      },
    },
    Table: {
      styles: {
        table: { borderColor: "var(--mantine-color-default-border)" },
        tr: { borderColor: "var(--mantine-color-default-border)" },
        th: {
          color: "var(--mantine-color-text)",
          borderColor: "var(--mantine-color-default-border)",
        },
        td: { borderColor: "var(--mantine-color-default-border)" },
      },
    },
    Accordion: {
      styles: {
        item: { borderColor: "var(--mantine-color-default-border)" },
        control: {
          backgroundColor: "var(--mantine-color-brand-0)",
          color: "var(--mantine-color-brand-9)",
        },
        chevron: {
          color: "var(--mantine-color-brand-5)",
        },
      },
    },
    SegmentedControl: {
      styles: {
        root: {
          backgroundColor: "var(--mantine-color-brand-1)",
          borderColor: "var(--mantine-color-default-border)",
        },
        label: {
          color: "var(--mantine-color-brand-7)",
        },
        indicator: {
          backgroundColor: "white",
        },
      },
    },
    TextInput: {
      styles: {
        input: { borderColor: "var(--mantine-color-default-border)" },
      },
    },
    PasswordInput: {
      styles: {
        input: { borderColor: "var(--mantine-color-default-border)" },
      },
    },
    Textarea: {
      styles: {
        input: { borderColor: "var(--mantine-color-default-border)" },
      },
    },
    Select: {
      styles: {
        input: { borderColor: "var(--mantine-color-default-border)" },
      },
    },
    Checkbox: {
      styles: {
        input: { borderColor: "var(--mantine-color-default-border)" },
      },
    },
    Paper: {
      styles: {
        root: { borderColor: "var(--mantine-color-default-border)" },
      },
    },
  },
});

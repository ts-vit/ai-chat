import type { CSSVariablesResolver } from "@mantine/core";

/**
 * CSS variables resolver: text, borders, dimmed colors from brand palette.
 * Adapts to light/dark color scheme.
 */
export const uniCssResolver: CSSVariablesResolver = () => ({
  variables: {},
  light: {
    "--mantine-color-text": "#72442a", // brand-9
    "--mantine-color-dimmed": "#a5653a", // brand-7
    "--mantine-color-default-border": "#f3c9a7", // brand-2
  },
  dark: {
    "--mantine-color-text": "#f3c9a7", // brand-2
    "--mantine-color-dimmed": "#e59557", // brand-4
    "--mantine-color-default-border": "#8b5432", // brand-8
  },
});

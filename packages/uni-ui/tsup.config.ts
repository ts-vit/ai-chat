import { defineConfig } from "tsup";

export default defineConfig({
  entry: ["src/index.ts"],
  format: ["esm"],
  dts: {
    tsconfig: "tsconfig.build.json",
  },
  sourcemap: true,
  clean: true,
  external: [
    "react",
    "react-dom",
    "react/jsx-runtime",
    "@mantine/core",
    "@mantine/hooks",
    "@mantine/notifications",
    "@tabler/icons-react",
    "react-markdown",
    "remark-gfm",
    "rehype-highlight",
  ],
  esbuildOptions(options) {
    options.jsx = "automatic";
  },
});

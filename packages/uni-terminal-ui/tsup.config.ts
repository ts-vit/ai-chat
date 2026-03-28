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
    "@tauri-apps/api",
    "@xterm/xterm",
    "@xterm/addon-fit",
    "@xterm/addon-web-links",
  ],
  esbuildOptions(options) {
    options.jsx = "automatic";
  },
});

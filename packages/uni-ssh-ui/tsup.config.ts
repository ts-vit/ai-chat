import { defineConfig } from "tsup";
import pkg from "./package.json";

const external = [
  ...Object.keys(pkg.dependencies || {}),
  ...Object.keys(pkg.peerDependencies || {}),
  "react/jsx-runtime",
];

export default defineConfig({
  entry: ["src/index.ts"],
  format: ["esm"],
  dts: {
    tsconfig: "tsconfig.build.json",
  },
  sourcemap: true,
  clean: true,
  external,
  esbuildOptions(options) {
    options.jsx = "automatic";
  },
});

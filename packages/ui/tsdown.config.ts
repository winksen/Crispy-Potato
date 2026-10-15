import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/index.ts"],
  format: "esm",
  platform: "neutral",
  dts: true,
  clean: true,
  // One output file per source file so consumers' bundlers can tree-shake per component.
  unbundle: true,
  copy: [{ from: "src/styles/theme.css", to: "dist" }],
});

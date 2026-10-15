import { fileURLToPath } from "node:url";
import type { StorybookConfig } from "@storybook/react-vite";
import tailwindcss from "@tailwindcss/vite";

const uiSrc = fileURLToPath(new URL("../../../packages/ui/src", import.meta.url));

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(ts|tsx)"],
  addons: ["@storybook/addon-a11y", "@storybook/addon-docs"],
  framework: "@storybook/react-vite",
  viteFinal: (viteConfig) => ({
    ...viteConfig,
    plugins: [...(viteConfig.plugins ?? []), tailwindcss()],
    resolve: {
      ...viteConfig.resolve,
      // Stories import the package by its public name (like a consumer would), but resolve to
      // source so edits hot-reload without a rebuild.
      alias: { ...viteConfig.resolve?.alias, "@crispy-potato/ui": `${uiSrc}/index.ts` },
    },
  }),
};

export default config;

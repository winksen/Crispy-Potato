import type { Preview } from "@storybook/react-vite";
import "./preview.css";

const preview: Preview = {
  parameters: {
    a11y: {
      // 'error' fails the story's a11y check in CI-style runs instead of only warning in the panel.
      test: "error",
    },
    controls: { matchers: { color: /(background|color)$/i } },
  },
  tags: ["autodocs"],
};

export default preview;

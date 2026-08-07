import type { Preview } from "@storybook/react-vite";

import "../src/app/styles/index.css";

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },
    docs: {
      toc: true,
    },
    layout: "fullscreen",
  },
};

export default preview;

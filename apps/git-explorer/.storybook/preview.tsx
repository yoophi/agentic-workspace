import type { Preview } from "@storybook/react-vite";

import { StorybookProviders } from "../src/shared/storybook/storybook-providers";
import "../src/app/styles/index.css";

const preview: Preview = {
  decorators: [
    (Story) => (
      <StorybookProviders>
        <div className="min-h-screen bg-background p-6 text-foreground">
          <Story />
        </div>
      </StorybookProviders>
    ),
  ],
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    docs: {
      toc: true,
    },
    layout: "fullscreen",
  },
};

export default preview;

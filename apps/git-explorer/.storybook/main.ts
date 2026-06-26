import path from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import type { StorybookConfig } from "@storybook/react-vite";
import { mergeConfig } from "vite";

const storybookDir = path.dirname(fileURLToPath(import.meta.url));

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(ts|tsx|mdx)"],
  addons: ["@storybook/addon-docs", "@storybook/addon-a11y"],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
  viteFinal: async (config) =>
    mergeConfig(config, {
      plugins: [tailwindcss()],
      resolve: {
        alias: {
          "@": path.resolve(storybookDir, "../src"),
          "@tauri-apps/api/core": path.resolve(storybookDir, "./mocks/tauri-core.ts"),
          "@tauri-apps/api/event": path.resolve(storybookDir, "./mocks/tauri-event.ts"),
          "@tauri-apps/plugin-dialog": path.resolve(storybookDir, "./mocks/tauri-dialog.ts"),
        },
      },
    }),
};

export default config;

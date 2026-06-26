import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  stories: ["../src/stories/**/*.stories.@(ts|tsx)"],
  addons: ["@storybook/addon-docs"],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
  viteFinal(config) {
    config.resolve ??= {};
    config.resolve.alias = {
      ...(Array.isArray(config.resolve.alias) ? {} : config.resolve.alias),
      "@": new URL("../src", import.meta.url).pathname,
    };
    return config;
  },
};

export default config;

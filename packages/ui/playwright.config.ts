import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests/browser",
  fullyParallel: true,
  use: { baseURL: "http://127.0.0.1:46107", viewport: { width: 1280, height: 800 } },
  webServer: {
    command: "pnpm --filter @yoophi/git-explorer exec vite preview --outDir storybook-static --host 127.0.0.1 --port 46107 --strictPort",
    url: "http://127.0.0.1:46107",
    reuseExistingServer: false,
    timeout: 30_000,
  },
});

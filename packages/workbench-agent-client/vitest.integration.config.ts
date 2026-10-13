import { defineConfig } from "vitest/config";
export default defineConfig({
  test: {
    include: ["src/**/*.integration.test.ts"],
    globalSetup: ["../workbench-client/src/test/integration/global-setup.ts"],
    testTimeout: 60_000,
    hookTimeout: 600_000,
    fileParallelism: false,
  },
});

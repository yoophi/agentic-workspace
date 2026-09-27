import { defineConfig } from "vitest/config";

// 043 T041·T042: 실제 042 서버(시험 host 프로세스)에 붙는 통합 suite. 기본 `test`에서는 돌지 않는다(`test:integration`).
export default defineConfig({
  test: {
    include: ["src/test/integration/**/*.integration.test.ts"],
    globalSetup: ["src/test/integration/global-setup.ts"],
    testTimeout: 60_000,
    hookTimeout: 600_000,
    fileParallelism: false,
  },
});

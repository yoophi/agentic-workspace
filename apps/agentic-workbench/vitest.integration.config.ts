import path from "path";
import { defineConfig } from "vitest/config";

// 043 T041: 화면 코드(네트워크 이벤트 계층·교환 원장)를 실제 042 서버(시험 host 프로세스)에 붙이는 통합 시험.
// 기본 `test`에는 섞이지 않게 `*.itest.ts`만 돈다.
export default defineConfig({
  resolve: { alias: { "@": path.resolve(__dirname, "./src") } },
  test: {
    include: ["src/**/*.itest.ts"],
    environment: "node",
    globalSetup: ["../../packages/workbench-client/src/test/integration/global-setup.ts"],
    testTimeout: 60_000,
    hookTimeout: 600_000,
    fileParallelism: false,
  },
});

import { defineConfig } from "vitest/config";

// 이 패키지의 테스트는 타입 수준이다: `*.test-d.ts`를 tsc로 검사해 OperationMap의 상관 타입을 고정한다(SC-005).
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    passWithNoTests: true,
    typecheck: {
      enabled: true,
      include: ["src/**/*.test-d.ts"],
      tsconfig: "./tsconfig.json",
    },
  },
});

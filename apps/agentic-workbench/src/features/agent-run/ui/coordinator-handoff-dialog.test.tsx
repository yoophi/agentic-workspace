import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(
  new URL("./coordinator-handoff-dialog.tsx", import.meta.url),
  "utf8",
);

describe("CoordinatorHandoffDialog", () => {
  it("requires an explicit summary and confirmation", () => {
    expect(SOURCE).toContain("Main Coordinator 인계");
    expect(SOURCE).toContain("previousRunId");
    expect(SOURCE).toContain("successorRunId");
    expect(SOURCE).toContain("확인 후 인계");
    expect(SOURCE).toContain("disabled={!summary.trim()}");
  });
});

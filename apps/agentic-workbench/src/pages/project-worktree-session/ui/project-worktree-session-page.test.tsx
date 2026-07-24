import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const source = readFileSync(new URL("./project-worktree-session-page.tsx", import.meta.url), "utf8");

describe("ProjectWorktreeSessionPage SDD prompt routing", () => {
  it("forwards SDD delivery mode to the active agent area", () => {
    expect(source).toContain("onSendSddPrompt={(request) =>");
    expect(source).toContain("delivery: request.delivery");
  });
});

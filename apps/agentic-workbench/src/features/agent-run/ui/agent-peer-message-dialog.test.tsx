import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(new URL("./agent-peer-message-dialog.tsx", import.meta.url), "utf8");

describe("AgentPeerMessageDialog", () => {
  it("offers exact peer targets and all prompt delivery modes", () => {
    expect(SOURCE).toContain("targetPanelId");
    expect(SOURCE).toContain('"send"');
    expect(SOURCE).toContain('"queue"');
    expect(SOURCE).toContain('"draft"');
    expect(SOURCE).toContain("16,384");
  });
});

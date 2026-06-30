import { describe, expect, it } from "vitest";

import {
  AUTO_REFRESH_INTERVAL_MS,
  findStaleMarkdownDocument,
} from "@yoophi/workspace-auto-refresh";

describe("markdown annotator auto reload integration", () => {
  it("uses the shared 3 second refresh policy for active markdown documents", () => {
    expect(AUTO_REFRESH_INTERVAL_MS).toBe(3_000);
  });

  it("marks unreadable active markdown documents as stale", () => {
    expect(
      findStaleMarkdownDocument({
        absolutePath: "/notes/missing.md",
        readable: false,
        now: 2,
      }),
    ).toEqual({
      kind: "markdown-document",
      id: "/notes/missing.md",
      reason: "unreadable",
      detectedAt: 2,
    });
  });
});

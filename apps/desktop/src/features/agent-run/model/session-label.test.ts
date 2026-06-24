import { describe, expect, it } from "vitest";

import type { ProviderSession } from "@/entities/agent-run/model";

import { formatSessionLabel } from "./session-label";

function makeSession(overrides: Partial<ProviderSession>): ProviderSession {
  return {
    agentId: "codex",
    id: "0123456789abcdef",
    cwd: null,
    title: null,
    file: "/tmp/x.jsonl",
    messageCount: 0,
    createdAt: null,
    updatedAt: null,
    model: null,
    branch: null,
    source: null,
    ...overrides,
  };
}

describe("formatSessionLabel", () => {
  it("uses the title when present", () => {
    const label = formatSessionLabel(makeSession({ title: "Fix the bug", messageCount: 3 }));
    expect(label).toContain("Fix the bug");
    expect(label).toContain("3 msgs");
  });

  it("falls back to the first 8 chars of the id when no title", () => {
    const label = formatSessionLabel(makeSession({ title: null, id: "abcdef1234567890" }));
    expect(label).toContain("abcdef12");
    expect(label).not.toContain("abcdef1234567890");
  });

  it("treats a blank title as missing", () => {
    const label = formatSessionLabel(makeSession({ title: "   ", id: "abcdef1234567890" }));
    expect(label).toContain("abcdef12");
  });

  it("omits the timestamp when updatedAt is null or invalid", () => {
    expect(formatSessionLabel(makeSession({ title: "T", messageCount: 1, updatedAt: null }))).toBe(
      "T · 1 msgs",
    );
    expect(
      formatSessionLabel(makeSession({ title: "T", messageCount: 1, updatedAt: "not-a-date" })),
    ).toBe("T · 1 msgs");
  });

  it("appends a timestamp when updatedAt is valid", () => {
    const label = formatSessionLabel(
      makeSession({ title: "T", messageCount: 1, updatedAt: "2026-06-01T00:00:00Z" }),
    );
    expect(label.startsWith("T · 1 msgs · ")).toBe(true);
  });
});

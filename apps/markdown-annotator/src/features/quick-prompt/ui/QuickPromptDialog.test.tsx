import { describe, expect, it } from "vitest";
import { formatQuickPromptForAgent, type QuickPromptContext } from "@yoophi/markdown-annotation-core";
import { standaloneQuickPromptTarget } from "../model/quick-prompt-target";

const context: QuickPromptContext = {
  id: "ctx-1",
  scope: {
    kind: "selection",
    label: "선택 영역",
    blockId: "b1",
    startLine: 1,
    endLine: 1,
  },
  sourceLabel: "note.md",
  documentPath: "/tmp/note.md",
  documentRevisionLabel: "rev-1",
  content: "Selected text",
  originalLength: 13,
  includedLength: 13,
  lengthState: "complete",
};

describe("QuickPromptDialog", () => {
  it("preserves selected context when formatting editable prompt text", () => {
    const prompt = formatQuickPromptForAgent({ promptText: "Explain this", context });

    expect(prompt).toContain("선택 영역");
    expect(prompt).toContain("Selected text");
    expect(prompt).toContain("Explain this");
    expect(standaloneQuickPromptTarget.availability).toBe("unavailable");
  });

  it("keeps reduced context and blocked target details available", () => {
    const reducedContext = {
      ...context,
      lengthState: "reduced" as const,
      includedLength: 8,
      originalLength: 20,
      reductionReason: "Too long",
    };
    const prompt = formatQuickPromptForAgent({ promptText: "Review", context: reducedContext });

    expect(prompt).toContain("reduced (8/20 chars)");
    expect(prompt).toContain("Too long");
    expect(standaloneQuickPromptTarget.unavailableReason).toContain("연결된 agent 대상이 없습니다.");
  });
});

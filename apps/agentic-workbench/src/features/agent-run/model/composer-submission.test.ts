import { describe, expect, it } from "vitest";

import { MAX_ORCHESTRATION_PROMPT_BYTES } from "@/entities/agent-orchestration";

import { decideComposerSubmission, noticeForFailure } from "./composer-submission";

const base = { disabled: false, targetsValid: true };

describe("decideComposerSubmission", () => {
  it("sends a trimmed message when the composer is usable", () => {
    expect(decideComposerSubmission({ ...base, message: "  조사해줘  " })).toEqual({
      kind: "send",
      message: "조사해줘",
    });
  });

  it("ignores empty text, a disabled composer, and an invalid target selection", () => {
    expect(decideComposerSubmission({ ...base, message: "   " }).kind).toBe("ignored");
    expect(
      decideComposerSubmission({ ...base, disabled: true, message: "조사해줘" }).kind,
    ).toBe("ignored");
    expect(
      decideComposerSubmission({ ...base, targetsValid: false, message: "조사해줘" }).kind,
    ).toBe("ignored");
  });

  // FR-044: refuse before sending so no target receives a partial dispatch.
  it("blocks a prompt one byte over the limit and allows one exactly at the limit", () => {
    const atLimit = decideComposerSubmission({
      ...base,
      message: "a".repeat(MAX_ORCHESTRATION_PROMPT_BYTES),
    });
    const overLimit = decideComposerSubmission({
      ...base,
      message: "a".repeat(MAX_ORCHESTRATION_PROMPT_BYTES + 1),
    });

    expect(atLimit.kind).toBe("send");
    expect(overLimit.kind).toBe("blocked");
    if (overLimit.kind !== "blocked") throw new Error("expected blocked");
    expect(overLimit.notice.reason).toContain("16KiB");
    expect(overLimit.notice.reason).toContain("1");
  });

  it("measures the trimmed message in UTF-8 bytes", () => {
    // "한" is 3 UTF-8 bytes, so this exceeds the byte limit while the character count
    // stays far below it. Counting characters instead of bytes would wrongly allow it.
    const multiByte = "한".repeat(Math.ceil(MAX_ORCHESTRATION_PROMPT_BYTES / 3) + 1);

    expect(multiByte.length).toBeLessThan(MAX_ORCHESTRATION_PROMPT_BYTES);
    expect(decideComposerSubmission({ ...base, message: multiByte }).kind).toBe("blocked");
  });

  it("does not report a size block for whitespace padding alone", () => {
    const padded = `  ${"a".repeat(MAX_ORCHESTRATION_PROMPT_BYTES)}  `;

    expect(decideComposerSubmission({ ...base, message: padded }).kind).toBe("send");
  });
});

describe("noticeForFailure", () => {
  // FR-022: the user must be able to tell "no Main run" from "Main is busy".
  it("separates an absent Main run from a busy Main run", () => {
    const inactive = noticeForFailure({
      code: "coordinatorInactive",
      message: "활성 Main Coordinator 실행이 없습니다.",
      retryable: false,
    });
    const busy = noticeForFailure({
      code: "coordinatorBusy",
      message: "Main이 아직 받을 수 없습니다.",
      retryable: true,
    });

    expect(inactive.reason).not.toEqual(busy.reason);
    expect(inactive.nextAction).toContain("실행을 시작");
    expect(busy.nextAction).toContain("기다린");
  });

  it("always carries a reason so a rejection is never silent", () => {
    const notice = noticeForFailure({ code: "unknown", message: "", retryable: false });

    expect(notice.reason.trim()).not.toHaveLength(0);
  });
});

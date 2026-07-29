import { describe, expect, it } from "vitest";

import {
  checkPromptSize,
  describeOrchestrationFailure,
  describePromptSizeViolation,
} from "./failure-guidance";
import { MAX_ORCHESTRATION_PROMPT_BYTES } from "./types";

describe("describeOrchestrationFailure", () => {
  it("distinguishes an absent Main run from a busy Main run (FR-022)", () => {
    const inactive = describeOrchestrationFailure({
      code: "coordinatorInactive",
      message: "활성 Main Coordinator 실행이 없습니다.",
      retryable: false,
    });
    const busy = describeOrchestrationFailure({
      code: "coordinatorBusy",
      message: "Main이 보고 통지를 아직 받을 수 없습니다.",
      retryable: true,
    });

    expect(inactive.reason).not.toEqual(busy.reason);
    expect(inactive.nextAction).not.toEqual(busy.nextAction);
    expect(inactive.nextAction).toContain("실행을 시작");
    expect(busy.nextAction).toContain("기다린");
    expect(inactive.retryable).toBe(false);
    expect(busy.retryable).toBe(true);
  });

  it("always provides a reason and keeps retryability from the backend", () => {
    const codes = [
      "coordinatorInactive",
      "coordinatorBusy",
      "capacityExceeded",
      "invalidInput",
      "revisionConflict",
      "duplicateConflict",
      "readOnlyViolation",
      "runtimeLost",
      "workerUnavailable",
      "scopeMismatch",
      "unauthorized",
      "somethingNewFromBackend",
    ];

    for (const code of codes) {
      const guidance = describeOrchestrationFailure({
        code,
        message: "백엔드 메시지",
        retryable: true,
      });
      expect(guidance.code).toBe(code);
      expect(guidance.reason.trim()).not.toHaveLength(0);
      expect(guidance.retryable).toBe(true);
    }
  });

  it("falls back to the backend message for unknown codes", () => {
    const guidance = describeOrchestrationFailure({
      code: "brandNewCode",
      message: "구체적인 백엔드 사유",
      retryable: false,
    });

    expect(guidance.reason).toBe("구체적인 백엔드 사유");
    expect(guidance.nextAction).toBeNull();
  });

  it("keeps a reason even when the backend message is empty", () => {
    const guidance = describeOrchestrationFailure({
      code: "brandNewCode",
      message: "",
      retryable: false,
    });

    expect(guidance.reason.trim()).not.toHaveLength(0);
  });
});

describe("checkPromptSize", () => {
  it("accepts a prompt exactly at the limit and rejects one byte over (FR-044)", () => {
    const atLimit = "a".repeat(MAX_ORCHESTRATION_PROMPT_BYTES);
    const overLimit = "a".repeat(MAX_ORCHESTRATION_PROMPT_BYTES + 1);

    expect(checkPromptSize(atLimit).ok).toBe(true);
    expect(checkPromptSize(atLimit).bytes).toBe(MAX_ORCHESTRATION_PROMPT_BYTES);
    expect(checkPromptSize(overLimit).ok).toBe(false);
    expect(checkPromptSize(overLimit).bytes).toBe(MAX_ORCHESTRATION_PROMPT_BYTES + 1);
  });

  it("measures UTF-8 bytes rather than code units so multi-byte text is not undercounted", () => {
    // "한" is 3 UTF-8 bytes, so the character count alone would pass the limit.
    const multiByte = "한".repeat(MAX_ORCHESTRATION_PROMPT_BYTES / 3 + 1);

    const check = checkPromptSize(multiByte);

    expect(check.bytes).toBeGreaterThan(MAX_ORCHESTRATION_PROMPT_BYTES);
    expect(check.ok).toBe(false);
  });

  it("states the excess and the allowed range", () => {
    const notice = describePromptSizeViolation(
      checkPromptSize("a".repeat(MAX_ORCHESTRATION_PROMPT_BYTES + 10)),
    );

    expect(notice).toContain("16KiB");
    expect(notice).toContain("10");
  });
});

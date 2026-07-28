import {
  checkPromptSize,
  describeOrchestrationFailure,
  describePromptSizeViolation,
  type OrchestrationFailure,
} from "@/entities/agent-orchestration";

/** What the Composer shows after a blocked or rejected submission. */
export type ComposerNotice = {
  reason: string;
  nextAction: string | null;
};

/**
 * Outcome of pressing send.
 *
 * - `ignored`: nothing to send (empty text, disabled composer, or no valid target).
 * - `blocked`: the request is refused locally before any target is contacted (FR-044).
 * - `send`: the trimmed message may be dispatched.
 */
export type ComposerSubmission =
  | { kind: "ignored" }
  | { kind: "blocked"; notice: ComposerNotice }
  | { kind: "send"; message: string };

/**
 * Decides whether a composer submission may proceed. Pure so the size boundary and the
 * ignore rules are testable without a DOM.
 */
export function decideComposerSubmission(input: {
  message: string;
  disabled: boolean;
  targetsValid: boolean;
}): ComposerSubmission {
  if (input.disabled || !input.targetsValid) return { kind: "ignored" };
  const trimmed = input.message.trim();
  if (!trimmed) return { kind: "ignored" };
  const size = checkPromptSize(trimmed);
  if (!size.ok) {
    return {
      kind: "blocked",
      notice: { reason: describePromptSizeViolation(size), nextAction: null },
    };
  }
  return { kind: "send", message: trimmed };
}

/**
 * Turns a backend rejection into the notice shown next to the composer, preserving the
 * distinction between reason and next action (FR-022, FR-048).
 */
export function noticeForFailure(failure: OrchestrationFailure): ComposerNotice {
  const guidance = describeOrchestrationFailure(failure);
  return { reason: guidance.reason, nextAction: guidance.nextAction };
}

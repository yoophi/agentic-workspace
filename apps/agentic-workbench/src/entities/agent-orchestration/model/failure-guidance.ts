import { MAX_ORCHESTRATION_PROMPT_BYTES } from "./types";

/**
 * Backend failure shape. Mirrors `OrchestrationError` (`{ code, message, retryable }`)
 * without importing the API layer, so the guidance stays a pure model helper.
 */
export type OrchestrationFailure = {
  code: string;
  message: string;
  retryable: boolean;
};

/**
 * User-facing explanation of a rejected orchestration request (FR-048). `reason` says what
 * happened and `nextAction` says what the user can do about it; `null` means there is no
 * user action, only a system-side condition.
 */
export type OrchestrationFailureGuidance = {
  code: string;
  reason: string;
  nextAction: string | null;
  retryable: boolean;
};

const RETRY_LATER = "잠시 뒤 다시 시도하세요.";

/**
 * Maps a backend failure code to a distinguishable reason and next action. Callers MUST
 * branch on `code` rather than on message text (FR-022, FR-048).
 */
export function describeOrchestrationFailure(
  failure: OrchestrationFailure,
): OrchestrationFailureGuidance {
  const base = { code: failure.code, retryable: failure.retryable };
  switch (failure.code) {
    case "coordinatorInactive":
      return {
        ...base,
        reason: "Main Coordinator 실행이 없습니다.",
        nextAction: "Main 패널에서 실행을 시작한 뒤 다시 위임하세요.",
      };
    case "coordinatorBusy":
      return {
        ...base,
        reason: "Main Coordinator가 아직 요청을 받을 수 없습니다.",
        nextAction: `현재 응답이 끝날 때까지 기다린 뒤 다시 시도하세요.`,
      };
    case "capacityExceeded":
      return {
        ...base,
        reason: "동시 실행 또는 패널 상한에 도달했습니다.",
        nextAction: "실행이 끝나거나 패널을 닫은 뒤 다시 시도하세요.",
      };
    case "invalidInput":
      return {
        ...base,
        reason: failure.message,
        nextAction: "입력을 고친 뒤 다시 보내세요.",
      };
    case "revisionConflict":
      return {
        ...base,
        reason: "워크스페이스 상태가 그 사이에 바뀌었습니다.",
        nextAction: "최신 상태를 확인한 뒤 다시 시도하세요.",
      };
    case "duplicateConflict":
      return {
        ...base,
        reason: "같은 요청 식별자가 다른 내용으로 재사용되었습니다.",
        nextAction: "새 요청으로 다시 보내세요.",
      };
    case "readOnlyViolation":
      return {
        ...base,
        reason: "읽기 전용 경계를 위반했습니다.",
        nextAction: "쓰기가 필요한 작업은 사용자가 직접 수행하세요.",
      };
    case "runtimeLost":
      return {
        ...base,
        reason: "실행 연결이 끊어져 대화 기록을 신뢰할 수 없습니다.",
        nextAction: "작업 목록의 보고와 결과를 확인하세요.",
      };
    case "workerUnavailable":
      return {
        ...base,
        reason: "에이전트 실행을 사용할 수 없습니다.",
        nextAction: failure.retryable ? RETRY_LATER : null,
      };
    case "scopeMismatch":
    case "unauthorized":
      return {
        ...base,
        reason: "이 창에서 허용되지 않는 요청입니다.",
        nextAction: null,
      };
    default:
      return {
        ...base,
        reason: failure.message || "요청이 거부되었습니다.",
        nextAction: failure.retryable ? RETRY_LATER : null,
      };
  }
}

export type PromptSizeCheck = {
  ok: boolean;
  bytes: number;
  limit: number;
};

/**
 * Measures a prompt the same way the backend does: UTF-8 bytes against
 * `MAX_ORCHESTRATION_PROMPT_BYTES` (FR-044). Used to reject before sending so no target
 * receives a partial dispatch.
 */
export function checkPromptSize(
  message: string,
  limit: number = MAX_ORCHESTRATION_PROMPT_BYTES,
): PromptSizeCheck {
  const bytes = new TextEncoder().encode(message).length;
  return { ok: bytes <= limit, bytes, limit };
}

/** Human-readable notice for an over-limit prompt, stating the excess and allowed range. */
export function describePromptSizeViolation(check: PromptSizeCheck): string {
  const limitKiB = Math.floor(check.limit / 1024);
  const overBytes = check.bytes - check.limit;
  return `프롬프트가 ${limitKiB}KiB 상한을 ${overBytes.toLocaleString("ko-KR")}바이트 초과했습니다. ${check.bytes.toLocaleString("ko-KR")}바이트 중 ${check.limit.toLocaleString("ko-KR")}바이트까지만 보낼 수 있습니다.`;
}

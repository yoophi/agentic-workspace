// 043 T019: 네트워크 경로의 오류 문자열은 호환 경로(Tauri command)와 같다. 기대값은 compat Rust 규칙이다:
// - 일반: `fault.message` 그대로(`workbench_compat::fault_to_string`)
// - 교환: `details.exchangeCode`가 있으면 `{"code","message"}` JSON, 없으면 message(`exchange_command_error`)
// - orchestration: `details.orchestrationError`가 있으면 그 JSON 문자열, 없으면 message(`orchestration_fault_string`)
import { describe, expect, it } from "vitest";

import { faultToString } from "./fault-string";
import type { WorkbenchFault } from "./operation-map";

function fault(message: string, details?: unknown): WorkbenchFault {
  return {
    code: "conflict",
    message,
    retryable: false,
    outcome: "notApplied",
    requestId: "r-1",
    details: details as WorkbenchFault["details"],
  };
}

describe("faultToString", () => {
  it("returns the message verbatim for ordinary operations", () => {
    expect(faultToString(fault("Project name is required."))).toBe("Project name is required.");
    expect(faultToString(fault("x", { exchangeCode: "ignored" }))).toBe("x");
  });

  it("encodes exchange faults as the legacy {code, message} JSON only when an exchange code exists", () => {
    // Rust: serde_json::to_string(&json!({ "code": code, "message": message })) — 키 순서 code, message.
    expect(
      faultToString(
        fault("Panel run is inactive or owned by another window.", {
          exchangeCode: "staleSourceRun",
        }),
        "exchange",
      ),
    ).toBe('{"code":"staleSourceRun","message":"Panel run is inactive or owned by another window."}');
    expect(faultToString(fault("bench not found."), "exchange")).toBe("bench not found.");
  });

  it("returns the orchestration domain error JSON when present", () => {
    const orchestrationError = { code: "notFound", message: "Orchestration workspace is not bootstrapped." };
    expect(faultToString(fault("wrapped", { orchestrationError }), "orchestration")).toBe(
      '{"code":"notFound","message":"Orchestration workspace is not bootstrapped."}',
    );
    expect(faultToString(fault("plain"), "orchestration")).toBe("plain");
  });

  it("escapes like serde_json for exchange messages with quotes and non-ASCII", () => {
    expect(faultToString(fault('대상 "패널"이 닫혔습니다', { exchangeCode: "targetClosing" }), "exchange")).toBe(
      '{"code":"targetClosing","message":"대상 \\"패널\\"이 닫혔습니다"}',
    );
  });
});

// 호환 경로(Tauri command)와 같은 오류 문자열(043 T021). 화면은 이 문자열을 그대로 보여 준다 — 규칙은 compat Rust와 같다:
// `workbench_compat::fault_to_string`, `exchange_command_error`, `orchestration_fault_string`.
import type { WorkbenchFault } from "./operation-map";

export type FaultFlavor = "default" | "exchange" | "orchestration";

function detail(fault: WorkbenchFault, key: string): unknown {
  const details = fault.details as Record<string, unknown> | null | undefined;
  return details ? details[key] : undefined;
}

export function faultToString(fault: WorkbenchFault, flavor: FaultFlavor = "default"): string {
  if (flavor === "exchange") {
    const code = detail(fault, "exchangeCode");
    if (typeof code === "string") {
      return JSON.stringify({ code, message: fault.message });
    }
  }
  if (flavor === "orchestration") {
    const error = detail(fault, "orchestrationError");
    if (error !== undefined && error !== null) {
      return JSON.stringify(error);
    }
  }
  return fault.message;
}

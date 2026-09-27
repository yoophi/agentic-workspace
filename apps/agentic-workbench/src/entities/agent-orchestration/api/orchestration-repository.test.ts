import { readFileSync } from "node:fs";

const COMPAT_TRANSPORT_SOURCE = readFileSync(
  new URL("../../../shared/api/transport/compat-transport.ts", import.meta.url),
  "utf8",
);
import { describe, expect, it } from "vitest";

import { parseOrchestrationError } from "./orchestration-repository";

const SOURCE = readFileSync(
  new URL("./orchestration-repository.ts", import.meta.url),
  "utf8",
);

describe("orchestration repository", () => {
  it("maps bootstrap, snapshot, Main binding and revision events", () => {
    for (const command of [
      "bootstrap_orchestration_workspace",
      "list_recoverable_orchestration_workspaces",
      "get_orchestration_workspace",
      "bind_main_coordinator_run",
      "delegate_orchestration_goal",
      "adopt_manual_orchestration_child",
      "list_orchestration_tasks",
      "collect_orchestration_reports",
      "set_orchestration_presentation",
      "replay_orchestration_runtime_events",
      "dispatch_orchestration_prompt",
      "respond_orchestration_input",
      "cancel_orchestration_task",
      "retry_orchestration_task",
      "reassign_orchestration_task",
      "handoff_orchestration_coordinator",
      "recover_orchestration_workspace",
      "send_orchestration_child_command",
    ]) {
      expect(SOURCE).toContain(`"${command}"`);
    }
    expect(SOURCE).toContain('"orchestration-workspace-updated"');
    expect(SOURCE).toContain('"orchestration-command-updated"');
    expect(SOURCE).toContain(
      '"orchestration-coordinator-notification-updated"',
    );
    // 043: 창 삽입(`<event>-fallback`) 수신은 호환 transport가 맡고, 저장소는 창 transport의 listen을 쓴다.
    expect(SOURCE).toContain("listen(");
    expect(COMPAT_TRANSPORT_SOURCE).toContain("fallback: `${event}-fallback`");
  });

  it("normalizes typed backend errors", () => {
    expect(
      parseOrchestrationError(
        '{"code":"revisionConflict","message":"stale","retryable":true}',
      ),
    ).toEqual({
      code: "revisionConflict",
      message: "stale",
      retryable: true,
    });
    expect(parseOrchestrationError(new Error("offline")).message).toBe(
      "offline",
    );
  });
});

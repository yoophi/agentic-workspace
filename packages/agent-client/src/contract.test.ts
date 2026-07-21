import { describe, expect, it } from "vitest";

import type { AgentRunRequest, RunEventEnvelope } from "./index";

// 계약 가드: Rust crate `acp-agent-core`의 domain(#[serde(rename_all="camelCase")])과
// 키 형태가 일치해야 한다. 타입에 맞춰 구성한 샘플의 JSON 키를 고정해 드리프트를 잡는다.

describe("agent-client contract (camelCase)", () => {
  it("AgentRunRequest keeps camelCase keys", () => {
    const request: AgentRunRequest = {
      runId: "run-1",
      goal: "이 자막을 요약",
      agentId: "claude-code",
      cwd: "/tmp/out",
      agentCommand: "claude",
      agentEnv: { FOO: "bar" },
      stdioBufferLimitMb: 8,
      autoAllow: false,
      resumePolicy: "fresh",
      permissionMode: "readOnly",
      contextSize: "default",
    };

    expect(Object.keys(request).sort()).toEqual(
      [
        "agentCommand",
        "agentEnv",
        "agentId",
        "autoAllow",
        "contextSize",
        "cwd",
        "goal",
        "permissionMode",
        "resumePolicy",
        "runId",
        "stdioBufferLimitMb",
      ].sort(),
    );
  });

  it("RunEventEnvelope wraps a camelCase run event", () => {
    const envelope: RunEventEnvelope = {
      runId: "run-1",
      event: { type: "agentMessage", text: "hello" },
    };

    expect(Object.keys(envelope).sort()).toEqual(["event", "runId"]);
    expect(envelope.event.type).toBe("agentMessage");
  });

  it("tool event uses camelCase toolCallId / fileChanges", () => {
    const envelope: RunEventEnvelope = {
      runId: "run-1",
      event: {
        type: "tool",
        toolCallId: "call_1",
        status: "completed",
        title: "Read file",
        locations: ["a.txt"],
        fileChanges: [
          {
            path: "a.txt",
            oldPath: null,
            kind: "modified",
            status: "completed",
            diff: null,
            content: null,
            binary: false,
            truncated: false,
            message: null,
          },
        ],
      },
    };

    const event = envelope.event;
    expect(event.type).toBe("tool");
    if (event.type === "tool") {
      expect(event.toolCallId).toBe("call_1");
      expect(event.fileChanges?.[0]?.oldPath).toBeNull();
    }
  });
});

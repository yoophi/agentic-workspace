import { describe, expect, it } from "vitest";

import { agentRunQueryKeys } from "./query-keys";
import { agentToolCommandCandidateQueryOptions } from "./query-options";

describe("agent tool command candidate query contract", () => {
  it("keys candidate queries by run, agent, working directory, and session mode", () => {
    expect(
      agentRunQueryKeys.toolCommandCandidates("run-1", "codex", "/repo", "reuse"),
    ).toEqual([
      "agent-tool-command-candidates",
      "run-1",
      "codex",
      "/repo",
      "reuse",
    ]);
  });

  it("keeps candidate data short-lived and non-retrying", () => {
    expect(agentToolCommandCandidateQueryOptions).toMatchObject({
      staleTime: 5_000,
      gcTime: 30_000,
      retry: false,
    });
  });
});

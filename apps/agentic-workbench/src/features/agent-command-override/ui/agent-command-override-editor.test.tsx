import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { AgentDescriptor } from "@/entities/agent-run/model/types";
import { createCommandOverrideDraft } from "@/features/agent-command-override/model/command-override-form";
import {
  AgentCommandOverrideEditor,
  agentRuntimeVersionLabel,
} from "./agent-command-override-editor";

const codexAgent: AgentDescriptor = {
  id: "codex",
  label: "Codex",
  command: "npx -y @agentclientprotocol/codex-acp@1.1.5",
  runtimeVersion: "1.1.5",
};

const claudeAgent: AgentDescriptor = {
  id: "claude-code",
  label: "Claude Code",
  command: "npx -y @agentclientprotocol/claude-agent-acp@0.60.0",
  runtimeVersion: "0.60.0",
};

describe("agentRuntimeVersionLabel", () => {
  it("renders the effective version in the settings profile card", () => {
    const html = renderToStaticMarkup(
      <AgentCommandOverrideEditor
        agents={[codexAgent, claudeAgent]}
        draft={createCommandOverrideDraft({})}
        onDraftChange={() => undefined}
        onSave={() => undefined}
      />,
    );

    expect(html).toContain("Codex ACP v1.1.5");
    expect(html).toContain("Claude ACP v0.60.0");
  });

  it("shows the pinned Codex ACP version", () => {
    expect(
      agentRuntimeVersionLabel({
        agent: codexAgent,
        profileCommand: "",
        globalCommand: "",
      }),
    ).toBe("Codex ACP v1.1.5");
  });

  it("shows the pinned version from the effective default command", () => {
    expect(
      agentRuntimeVersionLabel({
        agent: claudeAgent,
        profileCommand: "",
        globalCommand: "",
      }),
    ).toBe("Claude ACP v0.60.0");
  });

  it("warns when a global override uses the package without a version", () => {
    expect(
      agentRuntimeVersionLabel({
        agent: claudeAgent,
        profileCommand: "",
        globalCommand: "npx -y @agentclientprotocol/claude-agent-acp",
      }),
    ).toBe("Claude ACP 버전 미고정");
  });

  it("reports an unknown version for a custom wrapper command", () => {
    expect(
      agentRuntimeVersionLabel({
        agent: claudeAgent,
        profileCommand: "./scripts/run-claude-acp",
        globalCommand: "",
      }),
    ).toBe("Claude ACP 버전 확인 불가");
  });
});

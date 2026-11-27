import type { AgentTarget } from "@yoophi/markdown-annotation-core";

export const standaloneQuickPromptTarget: AgentTarget = {
  id: "standalone-unavailable",
  label: "Agent target unavailable",
  availability: "unavailable",
  unavailableReason: "현재 Markdown Annotator 단독 실행에서는 연결된 agent 대상이 없습니다.",
  ownerScope: "markdown-annotator",
};

export function createAvailableQuickPromptTarget(label: string): AgentTarget {
  return {
    id: "active-agent-target",
    label,
    availability: "available",
    ownerScope: "current-workspace-session",
  };
}

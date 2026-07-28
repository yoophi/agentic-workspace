import {
  MAIN_AGENT_NODE_ID,
  MAX_ORCHESTRATION_NODES,
  type AgentNode,
  type AgentNodeCreator,
  type AgentRoleProfile,
} from "./types";

export function createMainNode(createdAt: string): AgentNode {
  return {
    id: MAIN_AGENT_NODE_ID,
    kind: "main",
    parentNodeId: null,
    role: {
      id: "main-coordinator",
      name: "Main",
      responsibility: "직접 하위 에이전트의 작업을 조율합니다.",
      expectedOutput: "출처와 충돌을 구분한 종합 결과",
    },
    currentRunId: null,
    assignedTaskId: null,
    executionStatus: "unassigned",
    presentationStatus: "panel",
    promotionPolicy: "always",
    runtimeProfile: null,
    lastActivityAt: null,
    createdBy: "user",
    createdAt,
  };
}

export function createChildNode({
  id,
  role,
  createdAt,
  createdBy,
}: {
  id: string;
  role: AgentRoleProfile;
  createdAt: string;
  createdBy: AgentNodeCreator;
}): AgentNode {
  if (!id.trim() || id === MAIN_AGENT_NODE_ID) {
    throw new Error("A child requires a unique non-Main id.");
  }
  return {
    id,
    kind: "child",
    parentNodeId: MAIN_AGENT_NODE_ID,
    role,
    currentRunId: null,
    assignedTaskId: null,
    executionStatus: "unassigned",
    presentationStatus: "background",
    promotionPolicy: "onAttention",
    runtimeProfile: null,
    lastActivityAt: null,
    createdBy,
    createdAt,
  };
}

export function assertValidOrchestrationRelationships(
  nodes: AgentNode[],
  mainNodeId: string,
): void {
  if (nodes.length > MAX_ORCHESTRATION_NODES) {
    throw new Error("An orchestration workspace supports at most eight nodes.");
  }
  const ids = new Set(nodes.map((node) => node.id));
  const mains = nodes.filter((node) => node.kind === "main");
  if (
    ids.size !== nodes.length ||
    mainNodeId !== MAIN_AGENT_NODE_ID ||
    mains.length !== 1 ||
    mains[0]?.id !== MAIN_AGENT_NODE_ID ||
    mains[0]?.parentNodeId !== null
  ) {
    throw new Error("The workspace requires exactly one immutable Main.");
  }
  if (
    nodes.some(
      (node) =>
        node.kind === "child" && node.parentNodeId !== MAIN_AGENT_NODE_ID,
    )
  ) {
    throw new Error("Every agent must be a direct child of Main.");
  }
}

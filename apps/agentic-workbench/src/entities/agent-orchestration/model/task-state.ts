import type {
  AgentNode,
  PresentationStatus,
  TaskStatus,
} from "./types";

const taskTransitions: Record<TaskStatus, ReadonlySet<TaskStatus>> = {
  pending: new Set(["ready"]),
  ready: new Set(["running", "cancelled"]),
  running: new Set([
    "inputRequired",
    "blocked",
    "completed",
    "failed",
    "cancelled",
  ]),
  inputRequired: new Set(["running", "blocked", "cancelled"]),
  blocked: new Set(["ready", "cancelled"]),
  failed: new Set(["ready"]),
  completed: new Set(),
  cancelled: new Set(),
};

export function canTransitionTask(
  current: TaskStatus,
  next: TaskStatus,
): boolean {
  return current === next || taskTransitions[current].has(next);
}

function withPresentation(
  node: AgentNode,
  presentationStatus: PresentationStatus,
): AgentNode {
  return node.kind === "main"
    ? node
    : { ...node, presentationStatus };
}

export function promoteNode(node: AgentNode): AgentNode {
  return withPresentation(node, "panel");
}

export function detachNode(node: AgentNode): AgentNode {
  return withPresentation(node, "detached");
}

export function requireAttention(node: AgentNode): AgentNode {
  return withPresentation(node, "attentionRequired");
}

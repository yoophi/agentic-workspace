import type {
  AgentNode,
  OrchestrationSession,
} from "@/entities/agent-orchestration";
import type { RuntimeHydrationStatus } from "@/features/agent-run/model/agent-run-controller";

import { TaskActivityItem } from "./task-activity-item";

type TaskActivityRailProps = {
  session: OrchestrationSession;
  now?: number;
  onPromote?: (node: AgentNode) => void;
  onDetach?: (node: AgentNode) => void;
  onRespond?: (taskId: string, response: string) => void;
  onCancel?: (taskId: string) => void;
  onRetry?: (taskId: string) => void;
  onReassign?: (taskId: string, nodeId: string) => void;
  runtimeStates?: Record<string, RuntimeHydrationStatus>;
};

export function TaskActivityRail({
  session,
  now,
  onPromote,
  onDetach,
  onRespond,
  onCancel,
  onRetry,
  onReassign,
  runtimeStates = {},
}: TaskActivityRailProps) {
  const items = session.tasks.flatMap((task) => {
    const node = session.nodes.find((candidate) => candidate.id === task.assignedNodeId);
    return node && node.kind === "child" ? [{ task, node }] : [];
  });

  return (
    <aside
      className="w-72 shrink-0 overflow-y-auto border-l bg-muted/20 p-2"
      aria-label="하위 에이전트 작업"
    >
      <div className="mb-2 flex items-center justify-between px-1">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Agent Activity
        </h2>
        <span className="text-xs text-muted-foreground">{items.length}</span>
      </div>
      <div className="space-y-2">
        {items.map(({ task, node }) => (
          <TaskActivityItem
            key={task.id}
            task={task}
            node={node}
            reports={session.reports.filter((report) => report.taskId === task.id)}
            commands={session.commands.filter((command) => command.taskId === task.id)}
            runtimeHydrationStatus={
              node.executionStatus === "active" && node.currentRunId
                ? runtimeStates[node.currentRunId]
                : "runtimeLost"
            }
            now={now}
            onPromote={() => onPromote?.(node)}
            onDetach={() => onDetach?.(node)}
            onRespond={onRespond}
            onCancel={onCancel}
            onRetry={onRetry}
            reassignCandidates={session.nodes.filter(
              (candidate) =>
                candidate.kind === "child" &&
                candidate.id !== node.id &&
                candidate.assignedTaskId === null,
            )}
            onReassign={onReassign}
          />
        ))}
        {items.length === 0 && (
          <p className="rounded-md border border-dashed p-3 text-center text-xs text-muted-foreground">
            Main이 위임한 작업이 아직 없습니다.
          </p>
        )}
      </div>
    </aside>
  );
}

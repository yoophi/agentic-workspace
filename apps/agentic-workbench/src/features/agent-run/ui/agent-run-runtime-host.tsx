import { useEffect } from "react";

import {
  replayOrchestrationRuntimeEvents,
  type AgentNode,
} from "@/entities/agent-orchestration";
import { listenRunEvents } from "@/entities/agent-run/api/agent-run-repository";
import {
  AgentRunControllerRegistry,
  type AgentRunControllerState,
} from "@/features/agent-run/model/agent-run-controller";

type AgentRunRuntimeHostProps = {
  nodes: AgentNode[];
  registry: AgentRunControllerRegistry;
  onControllerStateChange?: (state: AgentRunControllerState) => void;
  onReplayGap?: () => void;
};

export async function hydrateAgentRunController(
  registry: AgentRunControllerRegistry,
  nodeId: string,
  runId: string,
) {
  const controller = registry.getOrCreate(nodeId, runId);
  controller.markLoading();
  const snapshot = await replayOrchestrationRuntimeEvents(
    runId,
    controller.snapshot.lastSequence,
  );
  controller.applySnapshot(snapshot);
  return { controller, snapshot };
}

/**
 * Keeps background run observation owned by the workspace rather than a panel.
 * ACP processes remain backend-owned; this host restores their bounded event
 * cursor when no visual panel is mounted.
 */
export function AgentRunRuntimeHost({
  nodes,
  registry,
  onControllerStateChange,
  onReplayGap,
}: AgentRunRuntimeHostProps) {
  useEffect(() => {
    let disposed = false;
    const unlistenLive = listenRunEvents((envelope) => {
      const controller = registry.get(envelope.runId);
      if (!controller) return;
      const event = envelope.event as {
        type?: string;
        status?: string;
      };
      controller.applyLive({
        runId: envelope.runId,
        sequence: controller.snapshot.lastSequence + 1,
        event: envelope.event,
        terminal:
          event.type === "error" ||
          (event.type === "lifecycle" &&
            ["completed", "cancelled"].includes(event.status ?? "")),
      });
    });
    const runs = nodes.flatMap((node) =>
      node.kind === "child" && node.currentRunId
        ? [{ nodeId: node.id, runId: node.currentRunId }]
        : [],
    );
    const unsubscribers = runs.map(({ nodeId, runId }) => {
      const controller = registry.getOrCreate(nodeId, runId);
      const unsubscribe = onControllerStateChange
        ? controller.subscribe(onControllerStateChange)
        : () => {};
      void hydrateAgentRunController(registry, nodeId, runId)
        .then(({ snapshot }) => {
          if (disposed) return;
          if (snapshot.gapDetected) onReplayGap?.();
        })
        .catch(() => {
          if (!disposed) {
            controller.markRuntimeLost();
            onReplayGap?.();
          }
        });
      return unsubscribe;
    });
    return () => {
      disposed = true;
      unlistenLive();
      for (const unsubscribe of unsubscribers) unsubscribe();
    };
  }, [
    nodes,
    onControllerStateChange,
    onReplayGap,
    registry,
  ]);

  return null;
}

import { useState } from "react";
import { AlertCircleIcon, PanelRightOpenIcon, PanelTopCloseIcon } from "lucide-react";

import type {
  AgentNode,
  OrchestrationTask,
  TaskCommand,
  TaskReport,
} from "@/entities/agent-orchestration";
import { describeOrchestrationFailure } from "@/entities/agent-orchestration";
import type { RuntimeHydrationStatus } from "@/features/agent-run/model/agent-run-controller";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

const statusLabels: Record<OrchestrationTask["status"], string> = {
  pending: "대기",
  ready: "준비",
  running: "실행 중",
  inputRequired: "입력 필요",
  blocked: "차단됨",
  completed: "완료",
  failed: "실패",
  cancelled: "취소됨",
};

type TaskActivityItemProps = {
  task: OrchestrationTask;
  node: AgentNode;
  reports: TaskReport[];
  commands?: TaskCommand[];
  runtimeHydrationStatus?: RuntimeHydrationStatus;
  now?: number;
  onPromote?: (nodeId: string) => void;
  onDetach?: (nodeId: string) => void;
  onRespond?: (taskId: string, response: string) => void;
  onCancel?: (taskId: string) => void;
  onRetry?: (taskId: string) => void;
  reassignCandidates?: AgentNode[];
  onReassign?: (taskId: string, nodeId: string) => void;
};

function formatElapsed(startedAt: string | null, now: number) {
  if (!startedAt) return "시작 전";
  const seconds = Math.max(0, Math.floor((now - Date.parse(startedAt)) / 1000));
  if (seconds < 60) return `${seconds}초`;
  const minutes = Math.floor(seconds / 60);
  return minutes < 60 ? `${minutes}분` : `${Math.floor(minutes / 60)}시간 ${minutes % 60}분`;
}

export function TaskActivityItem({
  task,
  node,
  reports,
  commands = [],
  runtimeHydrationStatus,
  now = Date.now(),
  onPromote,
  onDetach,
  onRespond,
  onCancel,
  onRetry,
  reassignCandidates = [],
  onReassign,
}: TaskActivityItemProps) {
  const [response, setResponse] = useState("");
  const [reassignNodeId, setReassignNodeId] = useState("");
  const attention =
    task.status === "inputRequired" ||
    task.status === "blocked" ||
    task.status === "failed";
  const latest = reports[reports.length - 1];
  const latestCommand = commands[commands.length - 1];
  const taskFailureNextAction = task.failure
    ? describeOrchestrationFailure(task.failure).nextAction
    : null;
  const runtimeLabel =
    runtimeHydrationStatus === "loading" || runtimeHydrationStatus === "idle"
      ? "ACP 이벤트 대기 중"
      : runtimeHydrationStatus === "gap"
        ? "일부 이전 이벤트를 복원할 수 없음"
        : runtimeHydrationStatus === "runtimeLost"
          ? "런타임 연결 유실"
          : runtimeHydrationStatus === "ready"
            ? "런타임 연결됨"
            : null;

  return (
    <article
      className="space-y-2 rounded-lg border bg-card p-3 text-card-foreground"
      aria-label={`${node.role.name}: ${task.title}`}
      data-status={task.status}
    >
      <div className="flex items-start gap-2">
        {attention && (
          <AlertCircleIcon
            className="mt-0.5 size-4 shrink-0 text-amber-500"
            aria-label="확인 필요"
          />
        )}
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-1.5">
            <strong className="truncate text-sm">{task.title}</strong>
            <Badge variant={attention ? "destructive" : "secondary"}>
              {statusLabels[task.status]}
            </Badge>
          </div>
          <p className="text-xs text-muted-foreground">
            {node.role.name} · {formatElapsed(task.startedAt, now)} · 시도 {task.attempt}
          </p>
        </div>
        {node.presentationStatus === "panel" ? (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`${node.role.name} 패널 분리`}
            onClick={() => onDetach?.(node.id)}
          >
            <PanelTopCloseIcon />
          </Button>
        ) : (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`${node.role.name} 패널로 열기`}
            onClick={() => onPromote?.(node.id)}
          >
            <PanelRightOpenIcon />
          </Button>
        )}
      </div>

      {latest && (
        <p className="line-clamp-2 text-xs" aria-label="최근 활동">
          {latest.summary}
        </p>
      )}

      {runtimeLabel && (
        <p className="text-xs text-muted-foreground" data-runtime-status={runtimeHydrationStatus}>
          {runtimeLabel}
        </p>
      )}

      {latestCommand && (
        <p
          className="text-xs text-muted-foreground"
          role="status"
          aria-live="polite"
          aria-atomic="true"
          data-command-status={latestCommand.status}
          data-command-retryable={
            latestCommand.failure ? String(latestCommand.failure.retryable) : undefined
          }
        >
          명령 {latestCommand.kind} · {latestCommand.status}
          {/* Keep the specific backend message, and state retryability in words so it does
              not depend on color or on reading the code (FR-048). */}
          {latestCommand.failure ? ` · ${latestCommand.failure.message}` : ""}
          {latestCommand.failure
            ? latestCommand.failure.retryable
              ? " · 재시도 가능"
              : " · 재시도 불가"
            : ""}
        </p>
      )}

      {task.failure && (
        <p
          className="text-xs text-destructive"
          data-task-failure-code={task.failure.code}
          data-task-failure-retryable={String(task.failure.retryable)}
        >
          실패 사유: {task.failure.message}
          {task.failure.retryable ? " · 재시도 가능" : " · 재시도 불가"}
          {taskFailureNextAction ? ` · ${taskFailureNextAction}` : ""}
        </p>
      )}

      {task.status === "inputRequired" && (
        <form
          className="flex gap-1.5"
          onSubmit={(event) => {
            event.preventDefault();
            const value = response.trim();
            if (value) {
              onRespond?.(task.id, value);
            }
          }}
        >
          <Input
            value={response}
            onChange={(event) => setResponse(event.target.value)}
            aria-label={`${task.title} 입력 응답`}
            placeholder="에이전트에 답변"
          />
          <Button type="submit" size="sm" disabled={!response.trim()}>
            응답
          </Button>
        </form>
      )}

      {(task.status === "failed" || task.status === "blocked") && (
        <div className="flex flex-wrap gap-1.5">
          <Button type="button" size="sm" variant="outline" onClick={() => onRetry?.(task.id)}>
            재시도
          </Button>
          <Button type="button" size="sm" variant="ghost" onClick={() => onCancel?.(task.id)}>
            취소
          </Button>
          {reassignCandidates.length > 0 && (
            <>
              <select
                className="h-8 rounded-md border bg-background px-2 text-xs"
                aria-label={`${task.title} 재할당 대상`}
                value={reassignNodeId}
                onChange={(event) => setReassignNodeId(event.target.value)}
              >
                <option value="">재할당 대상</option>
                {reassignCandidates.map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.role.name}
                  </option>
                ))}
              </select>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={!reassignNodeId}
                onClick={() => onReassign?.(task.id, reassignNodeId)}
              >
                재할당
              </Button>
            </>
          )}
        </div>
      )}
    </article>
  );
}

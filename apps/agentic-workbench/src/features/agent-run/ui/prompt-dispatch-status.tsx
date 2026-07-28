import type { PromptDispatchState } from "@/entities/agent-orchestration";
import { summarizePromptDispatch } from "@/entities/agent-orchestration";
import { Button } from "@/components/ui/button";

export function PromptDispatchStatus({
  state,
  onRetry,
}: {
  state: PromptDispatchState;
  onRetry?: (panelId: string) => void;
}) {
  if (!state.dispatchId) return null;
  const summary = summarizePromptDispatch(state);
  return (
    <div className="space-y-1 border-t px-3 py-2 text-xs" role="status" aria-live="polite">
      <p>
        전달 {summary.total} · 성공 {summary.succeeded} · 실패 {summary.failed} · 진행{" "}
        {summary.pending}
      </p>
      <ul className="flex flex-wrap gap-2">
        {state.targets.map((target) => (
          <li key={target.panelId} data-status={target.status}>
            {target.panelId}: {target.status}
            {target.error ? ` (${target.error})` : ""}
            {target.status === "failed" && onRetry && (
              <Button
                type="button"
                variant="link"
                size="sm"
                className="h-auto px-1 py-0 text-xs"
                onClick={() => onRetry(target.panelId)}
              >
                다시 시도
              </Button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

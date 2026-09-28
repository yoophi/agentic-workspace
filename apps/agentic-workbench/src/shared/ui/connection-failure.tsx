// 연결 실패 화면(044 T032, contracts/desktop-client.md §4): 외부 서버 모드에서 창이 Workbench 서버에 붙지 못하면 호환 경로 없이
// 이 화면으로 부팅한다. 이유(실행 파일 없음·기동 시간 초과·버전 불일치·권한 등)를 그대로 보여 주고 다시 시도하게 한다.
import { Button } from "@/components/ui/button";

export const CONNECTION_FAILURE_TITLE = "Workbench 서버에 연결하지 못했습니다";

export function ConnectionFailure({ reason, onRetry }: { reason: string; onRetry: () => void }) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-6">
      <div role="alert" className="flex max-w-md flex-col gap-3 rounded-lg border bg-card p-5 text-card-foreground shadow-sm">
        <h1 className="text-sm font-semibold">{CONNECTION_FAILURE_TITLE}</h1>
        <p className="break-words font-mono text-xs text-muted-foreground" data-connection-failure-reason>
          {reason}
        </p>
        <div>
          <Button size="sm" onClick={onRetry}>
            다시 시도
          </Button>
        </div>
      </div>
    </div>
  );
}

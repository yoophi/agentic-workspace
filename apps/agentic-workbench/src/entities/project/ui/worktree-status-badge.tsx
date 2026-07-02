import { Badge } from "@/components/ui/badge";
import type { GitWorktreeStatus } from "@/entities/project/model/git-worktree";

// worktree status badge의 단일 구현(specs/007 US1). "unknown"은 status 계산을
// 건너뛴(placeholder/includeStatus:false) 상태라 "확인 중"으로 표시한다.
export function WorktreeStatusBadge({ status }: { status: GitWorktreeStatus }) {
  if (status === "unknown") {
    return (
      <Badge variant="outline" className="shrink-0 text-muted-foreground">
        확인 중
      </Badge>
    );
  }

  return (
    <Badge variant={status === "dirty" ? "destructive" : "secondary"} className="shrink-0">
      {status}
    </Badge>
  );
}

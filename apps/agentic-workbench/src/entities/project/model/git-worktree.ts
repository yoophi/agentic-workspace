// "unknown"은 status 계산을 건너뛴 경우(includeStatus: false) 또는 목록 도착 전
// placeholder 상태다(specs/007 research R4, R5).
export type GitWorktreeStatus = "clean" | "prunable" | "dirty" | "unknown";

export type GitWorktree = {
  path: string;
  head?: string | null;
  branch?: string | null;
  status: GitWorktreeStatus;
  pruneReason?: string | null;
  canDelete: boolean;
};

export type GitWorktreeCreateInput = {
  path: string;
  branch: string;
  reference: string;
};

/**
 * URL의 worktree 경로만으로 세션 화면을 즉시 렌더링하기 위한 placeholder.
 * 목록 응답이 도착하면 실제 메타데이터로 교체된다(specs/007 research R4).
 */
export function createPlaceholderWorktree(path: string): GitWorktree {
  return {
    path,
    head: null,
    branch: null,
    status: "unknown",
    pruneReason: null,
    canDelete: false,
  };
}

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

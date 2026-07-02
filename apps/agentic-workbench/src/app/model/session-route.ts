import type { GitWorktree } from "@/entities/project/model/git-worktree";

const WORKTREE_PATH_PARAM = "worktreePath";

export function buildProjectWorktreeRoute(projectId: string, worktreePath: string) {
  const params = new URLSearchParams({ [WORKTREE_PATH_PARAM]: worktreePath });
  return `/projects/${projectId}/worktrees?${params.toString()}`;
}

export function readWorktreePath(searchParams: URLSearchParams) {
  return searchParams.get(WORKTREE_PATH_PARAM) ?? "";
}

export type SessionWorktreeResolution =
  | { kind: "missing-path" }
  | { kind: "placeholder"; worktree: GitWorktree }
  | { kind: "resolved"; worktree: GitWorktree }
  | { kind: "invalid" };

/**
 * URL의 worktree 경로로 세션 화면을 즉시 렌더링하기 위한 placeholder.
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

/**
 * 세션 route의 worktree 결정 규칙:
 * - 경로 없음 → missing-path
 * - 목록 로드 전 → placeholder(shell 우선 렌더링)
 * - 목록에 존재 → resolved(메타데이터 보강)
 * - 목록에 없음 → invalid(페이지 내부 오류 상태)
 */
export function resolveSessionWorktree({
  worktreePath,
  worktrees,
}: {
  worktreePath: string;
  worktrees: GitWorktree[] | undefined;
}): SessionWorktreeResolution {
  if (!worktreePath) {
    return { kind: "missing-path" };
  }

  if (worktrees === undefined) {
    return { kind: "placeholder", worktree: createPlaceholderWorktree(worktreePath) };
  }

  const worktree = worktrees.find((candidate) => candidate.path === worktreePath);

  if (!worktree) {
    return { kind: "invalid" };
  }

  return { kind: "resolved", worktree };
}

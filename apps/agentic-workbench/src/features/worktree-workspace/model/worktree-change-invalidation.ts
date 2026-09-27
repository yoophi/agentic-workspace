// Worktree 변경 알림 → query 무효화(선별 invalidation, contracts §5). 화면 패널이 쓰고, 043 재연결 재조회 신호(`kind: "git"`,
// `reason: "resync"`)도 같은 경로를 탄다 — 활성 탭 query는 즉시 refetch, 나머지는 stale 표시.
import type { QueryClient } from "@tanstack/react-query";

import { projectQueryKeys } from "@/entities/project/api/query-keys";
import { worktreeFileQueryKeys } from "@/entities/worktree-file/api/query-keys";
import { worktreeGitQueryKeys } from "@/entities/worktree-git/api/query-keys";

export function invalidateForWorktreeChange(
  queryClient: QueryClient,
  path: string,
  activeTab: string,
  kind: string | undefined,
) {
  // 파일 목록 전체 rescan(WalkDir)은 파일 트리가 화면에 있을 때만 즉시 필요하다.
  void queryClient.invalidateQueries({
    queryKey: worktreeFileQueryKeys.list(path),
    refetchType: activeTab === "git" ? "none" : "active",
  });
  void queryClient.invalidateQueries({
    queryKey: worktreeFileQueryKeys.textFiles(path),
    refetchType: activeTab === "git" ? "none" : "active",
  });
  void queryClient.invalidateQueries({
    queryKey: worktreeFileQueryKeys.speckit(path),
    refetchType: activeTab === "speckit" ? "active" : "none",
  });
  void queryClient.invalidateQueries({ queryKey: projectQueryKeys.worktreeChanges(path) });

  if (kind === "file") {
    return;
  }

  void queryClient.invalidateQueries({ queryKey: worktreeGitQueryKeys.history(path) });
  void queryClient.invalidateQueries({ queryKey: worktreeGitQueryKeys.graph(path) });
  void queryClient.invalidateQueries({ queryKey: ["worktree-git", "commit-detail", path] });
  void queryClient.invalidateQueries({ queryKey: ["worktree-git", "file-diff", path] });
}

import { useCallback, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { worktreeWorkspaceLayoutQueryKeys } from "@/entities/worktree-workspace-layout/api/query-keys";
import {
  getWorktreeWorkspaceLayout,
  saveWorktreeWorkspaceLayout,
} from "@/entities/worktree-workspace-layout/api/worktree-workspace-layout-repository";
import {
  emptyWorktreeWorkspaceLayout,
  withOuterPanelWidth,
  withPanelWidth,
  type WorkspacePanelId,
  type WorktreeWorkspaceLayout,
} from "@/entities/worktree-workspace-layout/model/types";

/// Worktree 하나의 레이아웃 레코드를 읽어 적용하고, 바깥·내부 B 폭 저장을 한곳에서 처리한다.
/// 저장은 항상 마지막 레코드 전체를 병합해 내보내므로 바깥 폭 저장이 내부 폭을 지우거나
/// 그 반대가 일어나지 않는다.
export function useWorktreeWorkspaceLayout(workingDirectory: string) {
  const queryClient = useQueryClient();
  const queryKey = worktreeWorkspaceLayoutQueryKeys.layout(workingDirectory);

  const query = useQuery({
    queryKey,
    queryFn: async () =>
      (await getWorktreeWorkspaceLayout(workingDirectory)) ??
      emptyWorktreeWorkspaceLayout(workingDirectory),
    // 사용자 조작으로만 바뀌는 값이므로 자동 refetch가 필요 없다.
    staleTime: Number.POSITIVE_INFINITY,
    refetchOnWindowFocus: false,
    retry: false,
  });

  useEffect(() => {
    if (!query.error) return;
    toast.error(`Workspace 레이아웃을 불러오지 못했습니다: ${describe(query.error)}`);
  }, [query.error]);

  // 읽기에 실패해도 기본 크기로 작업을 계속할 수 있어야 한다.
  const layout =
    query.data ?? (query.isError ? emptyWorktreeWorkspaceLayout(workingDirectory) : undefined);

  const persist = useCallback(
    (update: (current: WorktreeWorkspaceLayout) => WorktreeWorkspaceLayout) => {
      // 캐시의 마지막 레코드에서 시작해 한 필드만 바꿔 저장하므로, 바깥 폭 저장과
      // 내부 폭 저장이 서로의 값을 잃지 않는다.
      const key = worktreeWorkspaceLayoutQueryKeys.layout(workingDirectory);
      const current =
        queryClient.getQueryData<WorktreeWorkspaceLayout>(key) ??
        emptyWorktreeWorkspaceLayout(workingDirectory);
      const next = update(current);
      queryClient.setQueryData(key, next);
      void saveWorktreeWorkspaceLayout(next).catch((error) => {
        toast.error(`Workspace 레이아웃을 저장하지 못했습니다: ${describe(error)}`);
      });
    },
    [queryClient, workingDirectory],
  );

  const persistOuterWidth = useCallback(
    (widthPx: number) => persist((current) => withOuterPanelWidth(current, widthPx)),
    [persist],
  );

  const persistPanelWidth = useCallback(
    (panel: WorkspacePanelId, widthPx: number) =>
      persist((current) => withPanelWidth(current, panel, widthPx)),
    [persist],
  );

  return {
    layout,
    /// 저장된 레코드를 읽어 적용한 뒤에만 true. 이때부터 저장을 허용한다.
    hydrated: layout !== undefined,
    persistOuterWidth,
    persistPanelWidth,
  };
}

function describe(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}

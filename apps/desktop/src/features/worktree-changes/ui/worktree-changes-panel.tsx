import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { FileTextIcon, RefreshCwIcon } from "lucide-react";

import { listWorktreeChanges } from "@/entities/worktree-change/api/worktree-change-repository";
import { worktreeChangeQueryKeys } from "@/entities/worktree-change/api/query-keys";
import type { WorktreeChange } from "@/entities/worktree-change/model";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type WorktreeChangesPanelProps = {
  workingDirectory: string;
  isRunning: boolean;
};

const changeLabels: Record<WorktreeChange["changeType"], string> = {
  added: "Added",
  modified: "Modified",
  deleted: "Deleted",
  renamed: "Renamed",
  copied: "Copied",
  unmerged: "Unmerged",
  unknown: "Changed",
};

export function WorktreeChangesPanel({
  workingDirectory,
  isRunning,
}: WorktreeChangesPanelProps) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const changesQuery = useQuery({
    queryKey: worktreeChangeQueryKeys.changes(workingDirectory),
    queryFn: () => listWorktreeChanges(workingDirectory),
    enabled: Boolean(workingDirectory.trim()),
    refetchInterval: isRunning ? 2500 : false,
  });
  const changes = changesQuery.data ?? [];
  const selectedChange = useMemo(() => {
    if (changes.length === 0) {
      return null;
    }
    return changes.find((change) => change.path === selectedPath) ?? changes[0];
  }, [changes, selectedPath]);

  useEffect(() => {
    if (changes.length === 0) {
      setSelectedPath(null);
      return;
    }
    if (!selectedPath || !changes.some((change) => change.path === selectedPath)) {
      setSelectedPath(changes[0].path);
    }
  }, [changes, selectedPath]);

  return (
    <div className="flex flex-col rounded-lg border bg-background">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b px-3 py-2">
        <div className="flex min-w-0 items-center gap-2">
          <FileTextIcon className="size-4 text-muted-foreground" />
          <span className="text-sm font-medium">Changed files</span>
          <Badge variant="secondary">{changes.length}</Badge>
        </div>
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={changesQuery.isFetching}
          onClick={() => void changesQuery.refetch()}
        >
          <RefreshCwIcon
            data-icon="inline-start"
            className={cn(changesQuery.isFetching && "animate-spin")}
          />
          Refresh
        </Button>
      </div>

      {changesQuery.isError ? (
        <div className="px-3 py-3 text-sm text-destructive">
          변경 파일을 불러오지 못했습니다: {String(changesQuery.error)}
        </div>
      ) : changes.length === 0 ? (
        <div className="px-3 py-3 text-sm text-muted-foreground">
          현재 worktree에 표시할 변경 파일이 없습니다.
        </div>
      ) : (
        <div className="grid min-h-0 grid-cols-1 gap-0 lg:grid-cols-[minmax(0,260px)_minmax(0,1fr)]">
          <div className="max-h-72 overflow-auto border-b lg:border-b-0 lg:border-r">
            {changes.map((change) => (
              <button
                key={`${change.changeType}:${change.path}`}
                type="button"
                className={cn(
                  "flex w-full flex-col gap-1 border-b px-3 py-2 text-left last:border-b-0 hover:bg-muted/50",
                  selectedChange?.path === change.path && "bg-muted",
                )}
                onClick={() => setSelectedPath(change.path)}
              >
                <div className="flex min-w-0 items-center gap-2">
                  <Badge variant="outline">{changeLabels[change.changeType]}</Badge>
                  <span className="truncate font-mono text-xs">{change.path}</span>
                </div>
                <span className="line-clamp-2 text-xs text-muted-foreground">
                  {change.summary}
                </span>
              </button>
            ))}
          </div>

          <div className="flex min-h-0 flex-col">
            {selectedChange && (
              <>
                <div className="flex flex-wrap items-center gap-2 border-b px-3 py-2">
                  <Badge variant="outline">
                    {changeLabels[selectedChange.changeType]}
                  </Badge>
                  <span className="min-w-0 truncate font-mono text-xs">
                    {selectedChange.oldPath
                      ? `${selectedChange.oldPath} -> ${selectedChange.path}`
                      : selectedChange.path}
                  </span>
                  {selectedChange.truncated && (
                    <Badge variant="secondary">Preview truncated</Badge>
                  )}
                </div>
                <ChangeBody change={selectedChange} />
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function ChangeBody({ change }: { change: WorktreeChange }) {
  const content = change.diff ?? change.preview;
  if (change.binary || !content) {
    return (
      <div className="px-3 py-3 text-sm text-muted-foreground">
        {change.message ?? "이 파일의 diff 또는 preview를 표시할 수 없습니다."}
      </div>
    );
  }

  return (
    <pre className="max-h-96 overflow-auto whitespace-pre-wrap break-words px-3 py-3 font-mono text-xs leading-relaxed">
      {content}
    </pre>
  );
}

import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { GitCommitIcon, RefreshCwIcon } from "lucide-react";

import {
  getWorktreeChanges,
  getWorktreeFileDiff,
} from "@/entities/project/api/git-worktree-repository";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import type {
  GitChangedFile,
  GitChangedFileGroup,
} from "@/entities/project/model/git-worktree-changes";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { CodeBlock, CodeBlockCode } from "@/components/ui/code-block";
import { SystemMessage } from "@/components/ui/system-message";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type WorktreeChangesPanelProps = {
  workingDirectory: string;
  refreshSignal?: number;
};

const groupLabels: Record<GitChangedFileGroup, string> = {
  staged: "Staged",
  unstaged: "Unstaged",
  untracked: "Untracked",
  conflicted: "Conflicted",
};

const groupOrder: GitChangedFileGroup[] = ["conflicted", "staged", "unstaged", "untracked"];

export function WorktreeChangesPanel({
  workingDirectory,
  refreshSignal = 0,
}: WorktreeChangesPanelProps) {
  const [selectedPath, setSelectedPath] = useState("");
  const changesQuery = useQuery({
    queryKey: projectQueryKeys.worktreeChanges(workingDirectory),
    queryFn: () => getWorktreeChanges(workingDirectory),
  });
  const files = changesQuery.data?.files ?? [];
  const selectedFile = files.find((file) => file.path === selectedPath) ?? files[0] ?? null;
  const selectedDiffPath = selectedFile?.path ?? "";
  const diffQuery = useQuery({
    queryKey: projectQueryKeys.worktreeFileDiff(workingDirectory, selectedDiffPath),
    queryFn: () => getWorktreeFileDiff(workingDirectory, selectedDiffPath),
    enabled: Boolean(selectedDiffPath),
  });

  useEffect(() => {
    void changesQuery.refetch();
  }, [refreshSignal]);

  useEffect(() => {
    if (!selectedFile) {
      setSelectedPath("");
      return;
    }
    if (!files.some((file) => file.path === selectedPath)) {
      setSelectedPath(selectedFile.path);
    }
  }, [files, selectedFile, selectedPath]);

  const groupedFiles = useMemo(
    () =>
      groupOrder.map((group) => ({
        group,
        files: files.filter((file) => file.group === group),
      })),
    [files],
  );
  const totalCount = files.length;
  const isDirty = totalCount > 0;

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
          <div className="flex min-w-0 flex-col gap-1.5">
            <CardTitle className="flex items-center gap-2">
              <GitCommitIcon />
              Worktree changes
            </CardTitle>
            <CardDescription>
              Agent 실행 전후의 Git 변경사항과 선택 파일 diff를 확인합니다.
            </CardDescription>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={changesQuery.isFetching}
            onClick={() => void changesQuery.refetch()}
          >
            <RefreshCwIcon data-icon="inline-start" />
            새로고침
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {changesQuery.error ? (
          <SystemMessage variant="error" fill>
            {String(changesQuery.error)}
          </SystemMessage>
        ) : (
          <>
            <div className="flex flex-wrap gap-2">
              <Badge variant={isDirty ? "destructive" : "secondary"}>
                {isDirty ? `${totalCount} changed` : "Clean"}
              </Badge>
              <Badge variant="outline">Staged {changesQuery.data?.stagedCount ?? 0}</Badge>
              <Badge variant="outline">Unstaged {changesQuery.data?.unstagedCount ?? 0}</Badge>
              <Badge variant="outline">Untracked {changesQuery.data?.untrackedCount ?? 0}</Badge>
              {(changesQuery.data?.conflictedCount ?? 0) > 0 && (
                <Badge variant="destructive">
                  Conflicted {changesQuery.data?.conflictedCount}
                </Badge>
              )}
            </div>

            {!isDirty ? (
              <SystemMessage fill>
                이 worktree에는 현재 Git 변경사항이 없습니다.
              </SystemMessage>
            ) : (
              <div className="grid min-h-0 gap-4 xl:grid-cols-[minmax(260px,0.35fr)_minmax(0,1fr)]">
                <div className="flex max-h-96 min-h-0 flex-col gap-3 overflow-auto rounded-md border p-3">
                  {groupedFiles.map(({ group, files }) =>
                    files.length > 0 ? (
                      <div key={group} className="flex flex-col gap-1.5">
                        <div className="text-xs font-medium text-muted-foreground">
                          {groupLabels[group]} {files.length}
                        </div>
                        {files.map((file) => (
                          <ChangedFileButton
                            key={`${file.group}:${file.path}`}
                            file={file}
                            selected={file.path === selectedDiffPath}
                            onSelect={() => setSelectedPath(file.path)}
                          />
                        ))}
                      </div>
                    ) : null,
                  )}
                </div>
                <div className="min-w-0 rounded-md border bg-background">
                  <div className="border-b px-3 py-2">
                    <EllipsisPopoverText
                      value={selectedDiffPath || "파일을 선택하세요"}
                      className="font-mono text-xs text-muted-foreground"
                      contentClassName="font-mono text-xs"
                    />
                  </div>
                  {diffQuery.error ? (
                    <div className="p-3">
                      <SystemMessage variant="error" fill>
                        {String(diffQuery.error)}
                      </SystemMessage>
                    </div>
                  ) : (
                    <CodeBlock className="max-h-[28rem] overflow-auto rounded-none border-0">
                      <CodeBlockCode
                        code={
                          diffQuery.isFetching
                            ? "diff를 불러오는 중입니다..."
                            : diffQuery.data?.diff || "diff가 없습니다."
                        }
                        language="diff"
                      />
                    </CodeBlock>
                  )}
                </div>
              </div>
            )}

            {isDirty && (
              <SystemMessage variant="action" fill>
                변경사항 검토 후 터미널에서 필요한 파일을 stage하고 commit을 만든 뒤 PR을 준비하세요.
              </SystemMessage>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}

function ChangedFileButton({
  file,
  selected,
  onSelect,
}: {
  file: GitChangedFile;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      className={cn(
        "flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted",
        selected && "bg-muted",
      )}
      onClick={onSelect}
    >
      <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-[0.65rem] text-muted-foreground">
        {(file.stagedStatus ?? " ")}{(file.unstagedStatus ?? " ")}
      </span>
      <EllipsisPopoverText
        value={file.oldPath ? `${file.oldPath} -> ${file.path}` : file.path}
        className="min-w-0 flex-1 font-mono text-xs"
        contentClassName="font-mono text-xs"
        focusable={false}
      />
    </button>
  );
}

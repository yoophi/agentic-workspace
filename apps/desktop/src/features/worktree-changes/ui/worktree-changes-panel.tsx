import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ChevronRightIcon, FileDiffIcon, Loader2Icon, RefreshCwIcon } from "lucide-react";

import { listWorktreeChanges } from "@/entities/agent-run/api/worktree-changes-repository";
import { agentRunQueryKeys } from "@/entities/agent-run/api/query-keys";
import type { WorktreeChange } from "@/entities/agent-run/model/types";
import {
  changeBadgeVariant,
  changeTypeLabel,
  previewForChange,
} from "@/features/worktree-changes/model/worktree-change-view";
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
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type WorktreeChangesPanelProps = {
  workingDirectory: string;
  isRunning: boolean;
};

export function WorktreeChangesPanel({
  workingDirectory,
  isRunning,
}: WorktreeChangesPanelProps) {
  const changesQuery = useQuery({
    queryKey: agentRunQueryKeys.worktreeChanges(workingDirectory),
    queryFn: () => listWorktreeChanges(workingDirectory),
    enabled: Boolean(workingDirectory.trim()),
  });
  const changes = changesQuery.data ?? [];

  // run이 끝나면(실행 중 → 유휴 전환) 변경 목록을 자동으로 다시 불러온다.
  const wasRunningRef = useRef(isRunning);
  const refetchRef = useRef(changesQuery.refetch);
  refetchRef.current = changesQuery.refetch;
  useEffect(() => {
    if (wasRunningRef.current && !isRunning) {
      void refetchRef.current();
    }
    wasRunningRef.current = isRunning;
  }, [isRunning]);

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
          <div className="flex flex-col gap-1.5">
            <CardTitle className="flex items-center gap-2">
              <FileDiffIcon />
              변경된 파일
              {changes.length > 0 && (
                <Badge variant="secondary">{changes.length}</Badge>
              )}
            </CardTitle>
            <CardDescription>
              현재 worktree에서 HEAD 대비 생성/수정/삭제된 파일을 확인합니다.
            </CardDescription>
          </div>
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={changesQuery.isFetching}
            onClick={() => void changesQuery.refetch()}
          >
            {changesQuery.isFetching ? (
              <Loader2Icon data-icon="inline-start" className="animate-spin" />
            ) : (
              <RefreshCwIcon data-icon="inline-start" />
            )}
            새로고침
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {changesQuery.isLoading ? (
          <div className="flex items-center gap-2 rounded-md border bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
            <Loader2Icon className="size-4 animate-spin" />
            변경 파일을 불러오는 중
          </div>
        ) : changesQuery.isError ? (
          <div className="rounded-md border border-destructive/40 bg-destructive/5 px-3 py-2 text-sm text-destructive">
            변경 파일을 불러오지 못했습니다: {String(changesQuery.error)}
          </div>
        ) : changes.length === 0 ? (
          <div className="grid place-items-center rounded-md border border-dashed bg-muted/30 px-3 py-6 text-sm text-muted-foreground">
            변경된 파일이 없습니다.
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            {changes.map((change) => (
              <ChangeRow key={`${change.changeType}:${change.path}`} change={change} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ChangeRow({ change }: { change: WorktreeChange }) {
  const [open, setOpen] = useState(false);
  const preview = previewForChange(change);

  return (
    <Collapsible
      open={open}
      onOpenChange={setOpen}
      className="overflow-hidden rounded-md border bg-background"
    >
      <CollapsibleTrigger asChild>
        <button
          type="button"
          className="flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-muted/50"
        >
          <ChevronRightIcon
            className={cn("size-4 shrink-0 transition-transform", open && "rotate-90")}
            aria-hidden
          />
          <Badge variant={changeBadgeVariant(change.changeType)}>
            {changeTypeLabel(change.changeType)}
          </Badge>
          <span className="min-w-0 flex-1">
            <EllipsisPopoverText
              value={
                change.oldPath ? `${change.oldPath} → ${change.path}` : change.path
              }
              className="font-mono text-xs"
              contentClassName="font-mono text-xs"
            />
          </span>
          {change.binary && (
            <span className="shrink-0 text-xs text-muted-foreground">binary</span>
          )}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="border-t">
          {change.truncated && (
            <p className="px-3 py-1.5 text-xs text-muted-foreground">
              내용이 너무 커서 일부만 표시합니다.
            </p>
          )}
          <ChangePreviewBody change={change} preview={preview} />
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

function ChangePreviewBody({
  change,
  preview,
}: {
  change: WorktreeChange;
  preview: ReturnType<typeof previewForChange>;
}) {
  if (preview.kind === "binary") {
    return (
      <p className="px-3 py-3 text-sm text-muted-foreground">
        binary 파일은 미리볼 수 없습니다.
      </p>
    );
  }

  if (preview.kind === "empty") {
    return (
      <p className="px-3 py-3 text-sm text-muted-foreground">
        {change.changeType === "deleted"
          ? "삭제된 파일입니다."
          : "표시할 내용이 없습니다."}
      </p>
    );
  }

  return (
    <div className="max-h-96 overflow-auto">
      <CodeBlock className="rounded-none border-0">
        <CodeBlockCode code={preview.text} language={preview.language} />
      </CodeBlock>
    </div>
  );
}

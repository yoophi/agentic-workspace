import { ArrowLeftIcon, BotIcon, FolderGit2Icon } from "lucide-react";

import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { Project } from "@/entities/project/model/types";
import { AgentRunPanel } from "@/features/agent-run/ui/agent-run-panel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type ProjectWorktreeSessionPageProps = {
  project: Project;
  worktree: GitWorktree;
  onBack: () => void;
};

export function ProjectWorktreeSessionPage({
  project,
  worktree,
  onBack,
}: ProjectWorktreeSessionPageProps) {
  return (
    <div className="flex h-[calc(100svh-3rem)] min-h-0 flex-col gap-4 overflow-hidden">
      <div className="shrink-0 flex min-w-0 items-center gap-3">
        <Button type="button" variant="ghost" className="shrink-0" onClick={onBack}>
          <ArrowLeftIcon data-icon="inline-start" />
          프로젝트
        </Button>
        <Badge variant="secondary" className="shrink-0">
          Worktree Session
        </Badge>
        <h1 className="min-w-0 truncate text-2xl font-semibold tracking-normal">
          {project.name}
        </h1>
        <p className="min-w-0 flex-1 truncate text-sm text-muted-foreground">
          선택한 worktree에서 ACP agentic coding 작업을 실행합니다.
        </p>
      </div>

      <div className="min-h-0 flex-1">
        <AgentRunPanel
          workingDirectory={worktree.path}
          scrollHeader={
            <>
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <FolderGit2Icon />
                    선택된 worktree
                  </CardTitle>
                  <CardDescription>ACP 프로세스의 기준 작업 디렉토리입니다.</CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(180px,0.25fr)_minmax(180px,0.25fr)]">
                  <Detail label="경로" value={worktree.path} mono />
                  <Detail label="브랜치" value={worktree.branch || "-"} />
                  <div className="flex flex-col gap-1">
                    <span className="text-sm font-medium">상태</span>
                    <Badge variant={worktree.status === "dirty" ? "destructive" : "secondary"} className="w-fit">
                      {worktree.status}
                    </Badge>
                  </div>
                </CardContent>
              </Card>

              <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                <BotIcon />
                ACP 실행
              </div>
            </>
          }
        />
      </div>
    </div>
  );
}

function Detail({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <span className="text-sm font-medium">{label}</span>
      <EllipsisPopoverText
        value={value}
        className={mono ? "font-mono text-sm text-muted-foreground" : "text-sm text-muted-foreground"}
        contentClassName={mono ? "font-mono text-xs" : undefined}
      />
    </div>
  );
}

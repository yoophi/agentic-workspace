import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeftIcon,
  FolderIcon,
  GitBranchIcon,
  PencilIcon,
  Trash2Icon,
} from "lucide-react";

import { listGitRemotes } from "@/entities/project/api/git-remote-repository";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import type { Project } from "@/entities/project/model/types";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import {
  ProjectWorktreeCard,
  type OpenWorktreeMode,
} from "@/features/project-worktree/ui/project-worktree-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type ProjectDetailPageProps = {
  project: Project;
  onBack: () => void;
  onEditProject: (project: Project) => void;
  onDeleteProject: (project: Project) => void;
  onOpenWorktree: (worktree: GitWorktree, mode: OpenWorktreeMode) => void;
};

export function ProjectDetailPage({
  project,
  onBack,
  onEditProject,
  onDeleteProject,
  onOpenWorktree,
}: ProjectDetailPageProps) {
  const gitRemotesQuery = useQuery({
    queryKey: projectQueryKeys.gitRemotes(project.workingDirectory),
    queryFn: () => listGitRemotes(project.workingDirectory),
  });
  const gitRemotes = gitRemotesQuery.data ?? [];

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex flex-col gap-3">
          <Button
            type="button"
            variant="ghost"
            className="w-fit"
            onClick={onBack}
          >
            <ArrowLeftIcon data-icon="inline-start" />
            목록
          </Button>
          <div className="flex flex-col gap-2">
            <Badge variant="secondary" className="w-fit">
              Project Detail
            </Badge>
            <div className="flex flex-col gap-1">
              <h1 className="text-2xl font-semibold tracking-normal">
                {project.name}
              </h1>
              <p className="max-w-3xl text-sm text-muted-foreground">
                프로젝트 상세 정보를 확인하고 수정할 수 있습니다.
              </p>
            </div>
          </div>
        </div>
        <div className="flex gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={() => onEditProject(project)}
          >
            <PencilIcon data-icon="inline-start" />
            수정
          </Button>
          <Button
            type="button"
            variant="destructive"
            onClick={() => onDeleteProject(project)}
          >
            <Trash2Icon data-icon="inline-start" />
            삭제
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FolderIcon />
            기본 정보
          </CardTitle>
          <CardDescription>JSON 저장소에 기록된 프로젝트 데이터입니다.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          <div className="grid gap-5 md:grid-cols-2">
            <DetailField label="프로젝트 이름" value={project.name} />
            <DetailField label="프로젝트 ID" value={project.id} mono />
          </div>
          <Separator />
          <DetailField
            label="작업 디렉토리"
            value={project.workingDirectory}
            mono
          />
          <Separator />
          <DetailField
            label="설명"
            value={project.description || "설명 없음"}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <GitBranchIcon />
            Git remote
          </CardTitle>
          <CardDescription>
            작업 디렉토리에서 `git remote -v`로 확인한 원격 저장소 정보입니다.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {gitRemotesQuery.error ? (
            <p className="rounded-md border bg-background px-3 py-2 text-sm text-destructive">
              {String(gitRemotesQuery.error)}
            </p>
          ) : gitRemotes.length === 0 ? (
            <Empty>
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <GitBranchIcon />
                </EmptyMedia>
                <EmptyTitle>
                  {gitRemotesQuery.isLoading
                    ? "Git remote 정보를 불러오는 중입니다."
                    : "Git remote 정보가 없습니다."}
                </EmptyTitle>
                <EmptyDescription>
                  선택한 작업 디렉토리가 git 저장소가 아니거나 remote가 설정되어
                  있지 않습니다.
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          ) : (
            <div className="overflow-x-auto">
              <Table className="table-fixed">
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[20%]">이름</TableHead>
                    <TableHead className="w-[40%]">Fetch URL</TableHead>
                    <TableHead className="w-[40%]">Push URL</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {gitRemotes.map((remote) => (
                    <TableRow key={remote.name}>
                      <TableCell className="font-medium">
                        {remote.name}
                      </TableCell>
                      <TableCell className="min-w-0 font-mono text-xs text-muted-foreground">
                        <EllipsisPopoverText
                          value={remote.fetchUrl || "-"}
                          contentClassName="font-mono text-xs"
                        />
                      </TableCell>
                      <TableCell className="min-w-0 font-mono text-xs text-muted-foreground">
                        <EllipsisPopoverText
                          value={remote.pushUrl || "-"}
                          contentClassName="font-mono text-xs"
                        />
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <ProjectWorktreeCard
        workingDirectory={project.workingDirectory}
        onOpenWorktree={onOpenWorktree}
      />
    </div>
  );
}

function DetailField({
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
        className={cn("text-sm text-muted-foreground", mono && "font-mono")}
        contentClassName={cn(mono && "font-mono text-xs")}
      />
    </div>
  );
}

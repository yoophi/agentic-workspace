import { FormEvent, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BotIcon,
  GitBranchPlusIcon,
  GitCommitIcon,
  RefreshCwIcon,
  Trash2Icon,
} from "lucide-react";

import {
  createGitWorktree,
  deleteGitWorktree,
  listGitWorktrees,
} from "@/entities/project/api/git-worktree-repository";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import type {
  GitWorktree,
  GitWorktreeCreateInput,
  GitWorktreeStatus,
} from "@/entities/project/model/git-worktree";
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
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { GitReferenceCombobox } from "./git-reference-combobox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type ProjectWorktreeCardProps = {
  workingDirectory: string;
  onOpenWorktree?: (worktree: GitWorktree) => void;
};

const emptyForm: GitWorktreeCreateInput = {
  path: "",
  branch: "",
  reference: "",
};

export function ProjectWorktreeCard({
  workingDirectory,
  onOpenWorktree,
}: ProjectWorktreeCardProps) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<GitWorktreeCreateInput>(emptyForm);
  const [deletingPath, setDeletingPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const queryKey = projectQueryKeys.gitWorktrees(workingDirectory);
  const worktreesQuery = useQuery({
    queryKey,
    queryFn: () => listGitWorktrees(workingDirectory),
  });
  const worktrees = worktreesQuery.data ?? [];

  const createWorktreeMutation = useMutation({
    mutationFn: (input: GitWorktreeCreateInput) =>
      createGitWorktree(workingDirectory, input),
    onSuccess: async () => {
      setForm(emptyForm);
      await queryClient.invalidateQueries({ queryKey });
    },
  });
  const deleteWorktreeMutation = useMutation({
    mutationFn: (path: string) => deleteGitWorktree(workingDirectory, path),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey });
    },
  });

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    try {
      await createWorktreeMutation.mutateAsync(form);
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  async function handleDelete(path: string) {
    setDeletingPath(path);
    setError(null);

    try {
      await deleteWorktreeMutation.mutateAsync(path);
    } catch (caughtError) {
      setError(String(caughtError));
    } finally {
      setDeletingPath(null);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="flex flex-col gap-1.5">
            <CardTitle className="flex items-center gap-2">
              <GitCommitIcon />
              Git worktree
            </CardTitle>
            <CardDescription>
              worktree 목록과 상태를 확인하고 새 worktree를 생성합니다.
            </CardDescription>
          </div>
          <Button
            type="button"
            variant="outline"
            onClick={() => void worktreesQuery.refetch()}
            disabled={worktreesQuery.isLoading}
          >
            <RefreshCwIcon data-icon="inline-start" />
            새로고침
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <form onSubmit={handleCreate} className="flex flex-col gap-4">
          <FieldGroup>
            <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(160px,0.35fr)_minmax(160px,0.35fr)_auto]">
              <Field>
                <FieldLabel htmlFor="worktree-path">새 worktree 경로</FieldLabel>
                <Input
                  id="worktree-path"
                  value={form.path}
                  onChange={(event) =>
                    setForm((currentForm) => ({
                      ...currentForm,
                      path: event.target.value,
                    }))
                  }
                  placeholder="비워두면 자동 생성"
                />
                <FieldDescription>
                  비워두면 프로젝트 상위의 worktrees 디렉토리 아래에 자동
                  생성됩니다.
                </FieldDescription>
              </Field>
              <Field>
                <FieldLabel htmlFor="worktree-branch">브랜치</FieldLabel>
                <Input
                  id="worktree-branch"
                  value={form.branch}
                  onChange={(event) =>
                    setForm((currentForm) => ({
                      ...currentForm,
                      branch: event.target.value,
                    }))
                  }
                  placeholder="feature/add-login"
                  autoCapitalize="none"
                  autoCorrect="off"
                  spellCheck={false}
                />
                <FieldDescription>
                  비워두면 랜덤한 브랜치명이 자동 생성됩니다.
                </FieldDescription>
              </Field>
              <Field>
                <FieldLabel htmlFor="worktree-reference">기준 ref</FieldLabel>
                <GitReferenceCombobox
                  workingDirectory={workingDirectory}
                  value={form.reference}
                  onValueChange={(reference) =>
                    setForm((currentForm) => ({
                      ...currentForm,
                      reference,
                    }))
                  }
                />
              </Field>
              <div className="flex items-end">
                <Button type="submit" disabled={createWorktreeMutation.isPending}>
                  <GitBranchPlusIcon data-icon="inline-start" />
                  {createWorktreeMutation.isPending ? "생성 중" : "생성"}
                </Button>
              </div>
            </div>
            {error && <FieldError>{error}</FieldError>}
          </FieldGroup>
        </form>

        {worktreesQuery.error ? (
          <p className="rounded-md border bg-background px-3 py-2 text-sm text-destructive">
            {String(worktreesQuery.error)}
          </p>
        ) : worktrees.length === 0 ? (
          <p className="rounded-md border bg-background px-3 py-2 text-sm text-muted-foreground">
            {worktreesQuery.isLoading
              ? "Git worktree 목록을 불러오는 중입니다."
              : "표시할 Git worktree가 없습니다."}
          </p>
        ) : (
          <div className="overflow-x-auto">
            <Table className="table-fixed">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[42%]">경로</TableHead>
                  <TableHead className="w-[18%]">브랜치</TableHead>
                  <TableHead className="w-[16%]">HEAD</TableHead>
                  <TableHead className="w-[16%]">상태</TableHead>
                  <TableHead className="w-24 text-right">작업</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {worktrees.map((worktree) => (
                  <TableRow key={worktree.path}>
                    <TableCell className="min-w-0 font-mono text-xs">
                      <EllipsisPopoverText
                        value={worktree.path}
                        contentClassName="font-mono text-xs"
                      />
                    </TableCell>
                    <TableCell className="min-w-0">
                      <EllipsisPopoverText value={worktree.branch || "-"} />
                    </TableCell>
                    <TableCell className="min-w-0 font-mono text-xs text-muted-foreground">
                      <EllipsisPopoverText
                        value={worktree.head || "-"}
                        contentClassName="font-mono text-xs"
                      />
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col gap-1">
                        <WorktreeStatusBadge status={worktree.status} />
                        {worktree.pruneReason && (
                          <EllipsisPopoverText
                            value={worktree.pruneReason}
                            className="text-xs text-muted-foreground"
                            contentClassName="text-xs"
                          />
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex justify-end gap-1">
                        {onOpenWorktree && (
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon-sm"
                            onClick={() => onOpenWorktree(worktree)}
                            aria-label={`${worktree.path} worktree 작업 화면 열기`}
                          >
                            <BotIcon />
                          </Button>
                        )}
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-sm"
                          disabled={!worktree.canDelete || deletingPath === worktree.path}
                          onClick={() => void handleDelete(worktree.path)}
                          aria-label={`${worktree.path} worktree 삭제`}
                        >
                          <Trash2Icon />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function WorktreeStatusBadge({ status }: { status: GitWorktreeStatus }) {
  const labelByStatus: Record<GitWorktreeStatus, string> = {
    clean: "변경사항없음",
    prunable: "prune 가능",
    dirty: "변경사항 있음",
  };

  return (
    <Badge variant={status === "dirty" ? "destructive" : "secondary"}>
      {labelByStatus[status]}
    </Badge>
  );
}

import { FolderKanbanIcon, PlusIcon, RefreshCwIcon } from "lucide-react";

import { sortProjectsByName } from "@/entities/project/lib/sort-projects";
import type { Project } from "@/entities/project/model/types";
import { ProjectTable } from "@/features/project-list/ui/project-table";
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
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";

type ProjectListPageProps = {
  projects: Project[];
  isLoading: boolean;
  onRefresh: () => void;
  onCreateProject: () => void;
  onSelectProject: (project: Project) => void;
  onEditProject: (project: Project) => void;
  onDeleteProject: (project: Project) => void;
};

export function ProjectListPage({
  projects,
  isLoading,
  onRefresh,
  onCreateProject,
  onSelectProject,
  onEditProject,
  onDeleteProject,
}: ProjectListPageProps) {
  const sortedProjects = sortProjectsByName(projects);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex flex-col gap-2">
          <Badge variant="secondary" className="w-fit">
            Project Store
          </Badge>
          <div className="flex flex-col gap-1">
            <h1 className="text-2xl font-semibold tracking-normal">
              프로젝트 관리
            </h1>
            <p className="max-w-2xl text-sm text-muted-foreground">
              프로젝트 이름, 작업 디렉토리, 설명을 JSON 파일에 저장합니다.
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={onRefresh}
            disabled={isLoading}
          >
            <RefreshCwIcon data-icon="inline-start" />
            새로고침
          </Button>
          <Button type="button" onClick={onCreateProject}>
            <PlusIcon data-icon="inline-start" />
            프로젝트 생성
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>프로젝트 목록</CardTitle>
          <CardDescription>
            총 {projects.length.toLocaleString("ko-KR")}개의 프로젝트가
            저장되어 있습니다.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {sortedProjects.length === 0 ? (
            <Empty>
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <FolderKanbanIcon />
                </EmptyMedia>
                <EmptyTitle>
                  {isLoading
                    ? "프로젝트를 불러오는 중입니다."
                    : "등록된 프로젝트가 없습니다."}
                </EmptyTitle>
                <EmptyDescription>
                  프로젝트를 생성하면 이 목록에서 상세 정보를 확인하거나 수정할
                  수 있습니다.
                </EmptyDescription>
              </EmptyHeader>
              {!isLoading && (
                <EmptyContent>
                  <Button type="button" onClick={onCreateProject}>
                    <PlusIcon data-icon="inline-start" />
                    프로젝트 생성
                  </Button>
                </EmptyContent>
              )}
            </Empty>
          ) : (
            <ProjectTable
              projects={sortedProjects}
              onSelectProject={onSelectProject}
              onEditProject={onEditProject}
              onDeleteProject={onDeleteProject}
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
}

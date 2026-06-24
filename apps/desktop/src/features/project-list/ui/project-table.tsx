import { ExternalLinkIcon, PencilIcon, Trash2Icon } from "lucide-react";

import type { Project } from "@/entities/project/model/types";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type ProjectTableProps = {
  projects: Project[];
  onSelectProject: (project: Project) => void;
  onEditProject: (project: Project) => void;
  onDeleteProject: (project: Project) => void;
};

export function ProjectTable({
  projects,
  onSelectProject,
  onEditProject,
  onDeleteProject,
}: ProjectTableProps) {
  return (
    <div className="overflow-x-auto">
      <Table className="table-fixed">
        <TableHeader>
          <TableRow>
            <TableHead className="w-[24%]">이름</TableHead>
            <TableHead className="w-[34%]">작업 디렉토리</TableHead>
            <TableHead className="w-[28%]">설명</TableHead>
            <TableHead className="w-36 text-right">작업</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {projects.map((project) => (
            <TableRow key={project.id}>
              <TableCell>
                <Button
                  type="button"
                  variant="link"
                  className="h-auto justify-start p-0 font-medium"
                  onClick={() => onSelectProject(project)}
                >
                  {project.name}
                </Button>
              </TableCell>
              <TableCell className="min-w-0 font-mono text-xs">
                <EllipsisPopoverText
                  value={project.workingDirectory}
                  contentClassName="font-mono text-xs"
                />
              </TableCell>
              <TableCell className="min-w-0 text-muted-foreground">
                <EllipsisPopoverText value={project.description || "설명 없음"} />
              </TableCell>
              <TableCell>
                <div className="flex justify-end gap-1">
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => onSelectProject(project)}
                    aria-label={`${project.name} 상세`}
                  >
                    <ExternalLinkIcon />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => onEditProject(project)}
                    aria-label={`${project.name} 수정`}
                  >
                    <PencilIcon />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => onDeleteProject(project)}
                    aria-label={`${project.name} 삭제`}
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
  );
}

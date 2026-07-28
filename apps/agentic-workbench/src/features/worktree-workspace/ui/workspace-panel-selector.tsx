import { FileIcon, FileTextIcon, FolderKanbanIcon, GitBranchIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  workspacePanelIds,
  type WorkspacePanelId,
} from "@/features/worktree-workspace/model/workspace-layout";

const labels: Record<WorkspacePanelId, string> = {
  git: "Git",
  files: "Files",
  markdown: "Markdown",
  speckit: "Speckit",
};

const icons: Record<WorkspacePanelId, typeof GitBranchIcon> = {
  git: GitBranchIcon,
  files: FileIcon,
  markdown: FileTextIcon,
  speckit: FolderKanbanIcon,
};

type WorkspacePanelSelectorProps = {
  selectedPanel: WorkspacePanelId | null;
  onSelect: (panel: WorkspacePanelId) => void;
};

/// 화면 가장 오른쪽 세로 제어 영역. 선택된 버튼을 다시 누르면 선택이 해제되므로
/// 탭 목록이 아닌 토글 버튼 그룹으로 노출한다. (research.md 결정 4)
export function WorkspacePanelSelector({
  selectedPanel,
  onSelect,
}: WorkspacePanelSelectorProps) {
  return (
    <div
      role="group"
      aria-label="Workspace 패널"
      className="flex w-8 shrink-0 flex-col border-l bg-background py-2"
    >
      {workspacePanelIds.map((panel) => {
        const Icon = icons[panel];
        const isSelected = selectedPanel === panel;

        return (
          <Button
            key={panel}
            type="button"
            variant={isSelected ? "secondary" : "ghost"}
            className={cn("h-24 w-8 rounded-none px-0", isSelected && "bg-secondary")}
            aria-label={`${labels[panel]} 패널`}
            aria-pressed={isSelected}
            onClick={() => onSelect(panel)}
          >
            {/* 식별 표시만 90도 회전한다. 접근 가능한 이름은 회전과 무관하게 유지된다. */}
            <span className="flex rotate-90 items-center gap-1 whitespace-nowrap text-xs">
              <Icon className="size-4" aria-hidden="true" />
              {labels[panel]}
            </span>
          </Button>
        );
      })}
    </div>
  );
}

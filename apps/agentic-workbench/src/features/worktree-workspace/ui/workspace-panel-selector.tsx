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
const icons = { git: GitBranchIcon, files: FileIcon, markdown: FileTextIcon, speckit: FolderKanbanIcon };

export function WorkspacePanelSelector({ selectedPanel, onSelect }: {
  selectedPanel: WorkspacePanelId | null;
  onSelect: (panel: WorkspacePanelId) => void;
}) {
  return <div aria-label="Workspace panels" className="flex w-8 shrink-0 flex-col border-l bg-background py-2">
    {workspacePanelIds.map((panel) => {
      const Icon = icons[panel];
      return <Button key={panel} type="button" variant={selectedPanel === panel ? "secondary" : "ghost"}
        className={cn("h-24 w-8 rounded-none px-0", selectedPanel === panel && "bg-secondary")}
        aria-label={`${labels[panel]} panel`} aria-pressed={selectedPanel === panel} onClick={() => onSelect(panel)}>
        <span className="flex rotate-90 items-center gap-1 whitespace-nowrap text-xs">
          <Icon className="size-4" aria-hidden="true" />
          {labels[panel]}
        </span>
      </Button>;
    })}
  </div>;
}

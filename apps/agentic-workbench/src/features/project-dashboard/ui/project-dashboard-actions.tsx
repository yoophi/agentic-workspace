import {
  FolderOpenIcon,
  FolderPlusIcon,
  GitBranchIcon,
  PlayIcon,
  RefreshCwIcon,
  SquareArrowOutUpRightIcon,
} from "lucide-react";

import type { DashboardAction } from "@/entities/project/model";
import { Button } from "@/components/ui/button";

type ProjectDashboardActionsProps = {
  actions: DashboardAction[];
  onAction: (action: DashboardAction) => void;
  compact?: boolean;
};

export function ProjectDashboardActions({
  actions,
  onAction,
  compact = false,
}: ProjectDashboardActionsProps) {
  if (actions.length === 0) {
    return null;
  }

  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      {actions.map((action) => (
        <Button
          key={action.id}
          type="button"
          variant={action.kind === "createProject" ? "default" : "outline"}
          size={compact ? "sm" : "default"}
          disabled={!action.enabled}
          title={action.disabledReason}
          onClick={() => onAction(action)}
        >
          <DashboardActionIcon action={action} />
          {action.label}
        </Button>
      ))}
    </div>
  );
}

function DashboardActionIcon({ action }: { action: DashboardAction }) {
  if (action.kind === "createProject") {
    return <FolderPlusIcon data-icon="inline-start" />;
  }

  if (action.kind === "openExistingProject" || action.kind === "openProject") {
    return <FolderOpenIcon data-icon="inline-start" />;
  }

  if (action.kind === "resumeSession") {
    return <PlayIcon data-icon="inline-start" />;
  }

  if (action.kind === "openWorktree") {
    return <GitBranchIcon data-icon="inline-start" />;
  }

  if (action.kind === "retry") {
    return <RefreshCwIcon data-icon="inline-start" />;
  }

  return <SquareArrowOutUpRightIcon data-icon="inline-start" />;
}

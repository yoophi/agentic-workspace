export type WorktreeChangeType =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "copied"
  | "unmerged"
  | "unknown";

export type WorktreeChange = {
  path: string;
  oldPath?: string | null;
  changeType: WorktreeChangeType;
  summary: string;
  diff?: string | null;
  preview?: string | null;
  binary: boolean;
  truncated: boolean;
  message?: string | null;
};

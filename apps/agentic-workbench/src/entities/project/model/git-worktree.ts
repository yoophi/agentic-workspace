export type GitWorktreeStatus = "clean" | "prunable" | "dirty";

export type GitWorktree = {
  path: string;
  head?: string | null;
  branch?: string | null;
  status: GitWorktreeStatus;
  pruneReason?: string | null;
  canDelete: boolean;
};

export type GitWorktreeCreateInput = {
  path: string;
  branch: string;
  reference: string;
};

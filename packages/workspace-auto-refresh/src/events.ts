export const WORKTREE_CHANGED_EVENT = "workspace://worktree-changed";
export const MARKDOWN_DOCUMENT_CHANGED_EVENT = "workspace://markdown-document-changed";

export type WorkspaceChangeKind = "file" | "git";

export type WorktreeChangedEvent = {
  workingDirectory: string;
  changedPath: string;
  kind: WorkspaceChangeKind;
};

export type MarkdownDocumentChangedEvent = {
  path: string;
};

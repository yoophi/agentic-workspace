export type WorktreeFileEntry = {
  name: string;
  path: string;
  relativePath: string;
  isDir: boolean;
  size: number;
  modifiedMs?: number | null;
};

export type WorktreeTextFile = {
  path: string;
  relativePath: string;
  content: string;
  size: number;
  truncated: boolean;
};

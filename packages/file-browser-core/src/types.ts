export type FileBrowserEntryKind = "directory" | "file";
export type FileBrowserChildState = "unknown" | "loading" | "loaded";
export type FileBrowserSortField = "name" | "path" | "modifiedAt";

export type FileBrowserEntry = {
  path: string;
  kind: FileBrowserEntryKind;
  modifiedAt?: string;
  size?: number;
  childState?: FileBrowserChildState;
};

export type FileBrowserSort = {
  by: FileBrowserSortField;
  direction: "asc" | "desc";
};

export type FileBrowserOptions = {
  expandedPaths: ReadonlySet<string>;
  directoriesFirst?: boolean;
  searchQuery?: string;
  sort?: FileBrowserSort;
  compressSingleDirectoryChains?: boolean;
};

export type FileBrowserMatchRange = { start: number; end: number };

export type FileBrowserRow = {
  id: string;
  path: string;
  label: string;
  kind: FileBrowserEntryKind;
  depth: number;
  expanded: boolean;
  hasChildren: boolean;
  childState?: FileBrowserChildState;
  modifiedAt?: string;
  size?: number;
  matchRanges: readonly FileBrowserMatchRange[];
  chainPaths: readonly string[];
};

import type { FileBrowserRow } from "@yoophi/file-browser-core";
import type { CSSProperties, ReactNode } from "react";

export type FileBrowserTreeProps = {
  rows: readonly FileBrowserRow[];
  selectedPath?: string | null;
  activePath?: string | null;
  height?: number;
  rowHeight?: number;
  overscan?: number;
  ariaLabel: string;
  className?: string;
  style?: CSSProperties;
  renderRow: (row: FileBrowserRow, state: { active: boolean; selected: boolean }) => ReactNode;
  onSelect: (row: FileBrowserRow) => void;
  onToggle: (row: FileBrowserRow) => void;
};

// 순수 model / lib (프레임워크 비의존)
export { cn } from "./lib/cn";
export { diffLineClassName, fileStatusClassName } from "./lib/styling";
export { type DiffLine, parseDiffLines, parseHunkHeader } from "./model/diff";
export {
  buildFileTree,
  buildFileTreeRows,
  type FileTreeFolderNode,
  type FileTreeRow,
  getFileFolderPaths,
} from "./model/file-tree";
export { graphSegmentPath, laneX, refsByTarget } from "./model/graph-render";
export { combineGitCommitGraphPages } from "./model/commit-graph";

// React UI 컴포넌트 (react / react-dom / lucide-react는 peer)
export { GraphCell } from "./ui/graph-cell";
export { InfiniteLoadSentinel } from "./ui/infinite-load-sentinel";
export { DiffViewer } from "./ui/diff-viewer";
export { HistoryGraphView, type HistoryGraphViewProps } from "./ui/history-graph-view";
export { CommitDetailView, type CommitDetailViewProps } from "./ui/commit-detail-view";

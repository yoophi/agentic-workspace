// 그래프 페이지 결합 + ref 인덱싱(소비처에서 데이터 준비에 사용)
export {
  combineGitCommitGraphPages,
  combineGitCommitHistoryPages,
  getNextGitPageParam,
  initialGitPageParam,
  type GitPageParam,
} from "./model/commit-graph";
export { refsByTarget } from "./model/graph-render";
export { computeVirtualRowRange, type VirtualRowRange } from "./model/virtual-rows";

// React UI 컴포넌트 (react / react-dom / lucide-react는 peer)
export { InfiniteLoadSentinel } from "./ui/infinite-load-sentinel";
export { useVirtualRows } from "./ui/use-virtual-rows";
export { HistoryGraphView, type HistoryGraphViewProps } from "./ui/history-graph-view";
export { CommitDetailView, type CommitDetailViewProps } from "./ui/commit-detail-view";
export { DiffViewer } from "./ui/diff-viewer";
export { parseDiffLines, parseHunkHeader, type DiffLine } from "./model/diff";
export { WorktreeChangesView, type WorktreeChangesViewProps } from "./ui/worktree-changes-view";

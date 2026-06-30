// 그래프 페이지 결합 + ref 인덱싱(소비처에서 데이터 준비에 사용)
export { combineGitCommitGraphPages } from "./model/commit-graph";
export { refsByTarget } from "./model/graph-render";

// React UI 컴포넌트 (react / react-dom / lucide-react는 peer)
export { InfiniteLoadSentinel } from "./ui/infinite-load-sentinel";
export { HistoryGraphView, type HistoryGraphViewProps } from "./ui/history-graph-view";
export { CommitDetailView, type CommitDetailViewProps } from "./ui/commit-detail-view";

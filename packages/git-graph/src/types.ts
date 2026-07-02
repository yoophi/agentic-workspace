/**
 * git-explorer(정본)와 agentic-workbench가 공유하는 Git 데이터 타입.
 * Rust(git-core) 출력의 TypeScript 미러이며, history / commit-graph /
 * commit-detail / file-diff 응답 shape를 단일 소스로 정의한다.
 */

export type GitCommitSummary = {
  hash: string;
  message: string;
  author: string;
  date: string;
};

export type GitCommitPage = {
  offset: number;
  limit: number;
  /**
   * 전체 commit 수. 첫 페이지(offset 0, cursor 없음)에서만 채워지고 이후
   * 페이지는 null이다 — 소비자는 첫 페이지 값을 유지한다(AW specs/007 R8).
   */
  totalCount: number | null;
  hasMore: boolean;
  /** 요청 cursor가 현재 이력에 없어 목록을 처음부터 다시 로드해야 함을 표시. */
  cursorInvalidated?: boolean | null;
};

export type GitCommitHistory = {
  commits: GitCommitSummary[];
  page: GitCommitPage;
};

export type GitGraphCommit = {
  hash: string;
  shortHash: string;
  parents: string[];
  message: string;
  author: string;
  date: string;
  isHead: boolean;
  isMerge: boolean;
};

export type GitGraphRef = {
  name: string;
  target: string;
  kind: "localBranch" | "remoteBranch" | "tag";
};

export type GitGraphLayoutHints = {
  rowHeight: number;
  maxInitialLanes: number;
};

export type GitCommitGraph = {
  commits: GitGraphCommit[];
  refs: GitGraphRef[];
  page: GitCommitPage;
  layoutHints: GitGraphLayoutHints;
};

export type GitCommitFileChange = {
  path: string;
  status: string;
};

export type GitCommitDetail = GitCommitSummary & {
  files: GitCommitFileChange[];
};

export type GitFileDiff = {
  commitHash: string;
  path: string;
  content: string;
  isBinary: boolean;
  isTruncated: boolean;
};

export type GitCommitQueryOptions = {
  maxCount?: number;
  offset?: number;
  /** 마지막으로 받은 commit hash. 이력 재작성 감지와 count/refs 생략에 쓰인다. */
  cursor?: string;
  includedRefs?: string[];
  excludedRefs?: string[];
};

export type GitChangedFileGroup = "staged" | "unstaged" | "untracked" | "conflicted";

export type GitChangedFile = {
  path: string;
  oldPath?: string | null;
  stagedStatus?: string | null;
  unstagedStatus?: string | null;
  group: GitChangedFileGroup;
};

export type GitWorktreeChanges = {
  workingDirectory: string;
  files: GitChangedFile[];
  stagedCount: number;
  unstagedCount: number;
  untrackedCount: number;
  conflictedCount: number;
};

export type GitWorktreeFileDiff = {
  path: string;
  content: string;
  isBinary: boolean;
  isTruncated: boolean;
};

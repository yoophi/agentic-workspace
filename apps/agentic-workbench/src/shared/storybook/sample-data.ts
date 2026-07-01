import type {
  AgentDescriptor,
  ThreadGoal,
  ToolFileChange,
  WorktreeChange,
} from "@/entities/agent-run/model/types";
import type { GitBranch } from "@/entities/project/model/git-branch";
import type { GitRemote } from "@/entities/project/model/git-remote";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { GitWorktreeChanges } from "@/entities/project/model/git-worktree-changes";
import type {
  ChangeSummary,
  SessionSummary,
  WorktreeSummary,
} from "@/entities/project/model";
import type { Project } from "@/entities/project/model/types";
import { buildProjectDashboard } from "@/entities/project/lib/dashboard-summary";
import type { SavedPrompt } from "@/entities/saved-prompt/model/types";
import type {
  WorktreeFileEntry,
  WorktreeTextFile,
} from "@/entities/worktree-file/model/types";
import type {
  GitCommitDetail,
  GitCommitGraph,
  GitCommitHistory,
  GitFileDiff,
} from "@/entities/worktree-git/model/types";

export const sampleProjects: Project[] = [
  {
    id: "project-acp",
    name: "Agentic Workbench",
    workingDirectory: "/Users/yoophi/project/agentic-workbench",
    description: "Tauri 기반 ACP 데스크톱 클라이언트",
  },
  {
    id: "project-notes",
    name: "Private ZK",
    workingDirectory: "/Users/yoophi/docs/private-zk",
    description: "Logseq 개인 지식 그래프",
  },
];

export const sampleLongProjects: Project[] = [
  {
    id: "project-long-dashboard",
    name: "Agentic Workbench dashboard validation project with a very long display name",
    workingDirectory:
      "/Users/yoophi/project/worktrees/agentic-workspace/feature/project-dashboard-start-screen-with-long-path-layout-validation",
    description:
      "긴 프로젝트 이름, 긴 경로, 긴 설명이 시작화면의 action과 상태 영역을 침범하지 않는지 검증하기 위한 샘플입니다.",
  },
  ...sampleProjects,
];

const sampleSessionsByProjectId: Record<string, SessionSummary> = {
  "project-acp": {
    projectId: "project-acp",
    sessionId: "session-acp-dashboard",
    label: "Dashboard implementation",
    lastActivityLabel: "12분 전",
    lastActivityMs: 1_783_012_000_000,
    resumable: true,
    routeTarget: "/session/project-acp",
  },
  "project-notes": {
    projectId: "project-notes",
    sessionId: "session-notes-review",
    label: "Knowledge graph cleanup",
    lastActivityLabel: "어제",
    lastActivityMs: 1_782_900_000_000,
    resumable: true,
    routeTarget: "/session/project-notes",
  },
};

const sampleWorktreesByProjectId: Record<string, WorktreeSummary> = {
  "project-acp": {
    projectId: "project-acp",
    count: 3,
    activeCount: 2,
    primaryWorktreePath:
      "/Users/yoophi/project/worktrees/agentic-workbench/storybook",
    status: "ready",
  },
  "project-notes": {
    projectId: "project-notes",
    count: 1,
    activeCount: 1,
    status: "ready",
  },
};

const sampleChangesByProjectId: Record<string, ChangeSummary> = {
  "project-acp": {
    changedFileCount: 5,
    hasChanges: true,
    status: "ready",
  },
  "project-notes": {
    changedFileCount: 0,
    hasChanges: false,
    status: "ready",
  },
};

export const sampleProjectDashboard = buildProjectDashboard({
  projects: sampleProjects,
  isLoading: false,
  sessionsByProjectId: sampleSessionsByProjectId,
  worktreesByProjectId: sampleWorktreesByProjectId,
  changesByProjectId: sampleChangesByProjectId,
});

export const sampleProjectDashboardWithQuickActions = buildProjectDashboard({
  projects: sampleProjects,
  isLoading: false,
});

export const sampleEmptyProjectDashboard = buildProjectDashboard({
  projects: [],
  isLoading: false,
});

export const sampleLoadingProjectDashboard = buildProjectDashboard({
  projects: [],
  isLoading: true,
});

export const sampleErrorProjectDashboard = buildProjectDashboard({
  projects: [],
  isLoading: false,
  errorMessage: "프로젝트 저장소를 읽을 수 없습니다.",
});

export const samplePartialProjectDashboard = buildProjectDashboard({
  projects: sampleProjects,
  isLoading: false,
  sessionsByProjectId: sampleSessionsByProjectId,
  worktreesByProjectId: {
    ...sampleWorktreesByProjectId,
    "project-notes": {
      projectId: "project-notes",
      count: 0,
      activeCount: 0,
      status: "unavailable",
    },
  },
  changesByProjectId: {
    ...sampleChangesByProjectId,
    "project-notes": {
      status: "unavailable",
    },
  },
});

export const sampleLongContentProjectDashboard = buildProjectDashboard({
  projects: sampleLongProjects,
  isLoading: false,
  sessionsByProjectId: {
    "project-long-dashboard": {
      projectId: "project-long-dashboard",
      sessionId: "session-long-dashboard",
      label:
        "A very long resumable session label for validating dashboard action spacing",
      lastActivityLabel: "방금 전",
      lastActivityMs: 1_783_100_000_000,
      resumable: true,
      routeTarget: "/session/project-long-dashboard",
    },
  },
  worktreesByProjectId: {
    "project-long-dashboard": {
      projectId: "project-long-dashboard",
      count: 12,
      activeCount: 7,
      primaryWorktreePath:
        "/Users/yoophi/project/worktrees/agentic-workspace/feature/project-dashboard-start-screen-with-long-path-layout-validation",
      status: "ready",
    },
  },
  changesByProjectId: {
    "project-long-dashboard": {
      changedFileCount: 42,
      hasChanges: true,
      status: "ready",
    },
  },
});

export const sampleBranches: GitBranch[] = [
  { name: "main", isCurrent: true, isRemote: false },
  { name: "feature/storybook-atomic-design", isCurrent: false, isRemote: false },
  { name: "origin/main", isCurrent: false, isRemote: true },
];

export const sampleRemotes: GitRemote[] = [
  {
    name: "origin",
    fetchUrl: "git@github.com:yoophi/agentic-workbench.git",
    pushUrl: "git@github.com:yoophi/agentic-workbench.git",
  },
];

export const sampleWorktrees: GitWorktree[] = [
  {
    path: "/Users/yoophi/project/agentic-workbench",
    head: "a1b2c3d",
    branch: "main",
    status: "clean",
    canDelete: false,
  },
  {
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook",
    head: "d4e5f6a",
    branch: "feature/storybook-atomic-design",
    status: "dirty",
    canDelete: true,
  },
];

export const sampleAgents: AgentDescriptor[] = [
  { id: "codex", label: "Codex", command: "codex" },
  { id: "claude", label: "Claude Code", command: "claude" },
];

export const sampleSavedPrompts: SavedPrompt[] = [
  {
    id: "saved-continue",
    label: "Continue",
    prompt: "이전 결과를 바탕으로 계속 진행해주세요.",
  },
  {
    id: "saved-review",
    label: "Review",
    prompt: "현재 변경사항을 검토하고 위험한 부분을 알려주세요.",
  },
];

export const sampleWorktreeChanges: GitWorktreeChanges = {
  workingDirectory: "/Users/yoophi/project/worktrees/agentic-workbench/storybook",
  stagedCount: 1,
  unstagedCount: 1,
  untrackedCount: 1,
  conflictedCount: 0,
  files: [
    {
      path: "apps/agentic-workbench/src/features/worktree-change-review/ui/worktree-changes-panel.tsx",
      stagedStatus: "M",
      unstagedStatus: null,
      group: "staged",
    },
    {
      path: "apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx",
      stagedStatus: null,
      unstagedStatus: "M",
      group: "unstaged",
    },
    {
      path: "docs/change-review-notes.md",
      stagedStatus: "?",
      unstagedStatus: "?",
      group: "untracked",
    },
  ],
};

export const sampleAgentRunWorktreeChanges: WorktreeChange[] = [
  {
    path: "apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx",
    oldPath: null,
    changeType: "modified",
    binary: false,
    diff: [
      "diff --git a/apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx b/apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx",
      "index 1111111..2222222 100644",
      "--- a/apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx",
      "+++ b/apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx",
      "@@ -42,6 +42,7 @@ export function AgentRunPanel() {",
      "   const [items, setItems] = useState<TimelineItem[]>([]);",
      "+  const [changedFilesOpen, setChangedFilesOpen] = useState(true);",
    ].join("\n"),
    content: null,
    truncated: false,
  },
  {
    path: "apps/agentic-workbench/src/features/worktree-changes/ui/worktree-changes-panel.tsx",
    oldPath: null,
    changeType: "added",
    binary: false,
    diff: null,
    content: [
      "export function WorktreeChangesPanel() {",
      "  return <section>Changed files with diff preview</section>;",
      "}",
    ].join("\n"),
    truncated: false,
  },
  {
    path: "assets/screenshot.png",
    oldPath: null,
    changeType: "untracked",
    binary: true,
    diff: null,
    content: null,
    truncated: false,
  },
];

export const sampleAgentRunToolFileChanges: ToolFileChange[] = [
  {
    path: "apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx",
    oldPath: null,
    kind: "modified",
    status: "completed",
    binary: false,
    diff: [
      "--- a/apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx",
      "+++ b/apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx",
      "@@ -42,6 +42,8 @@ function ToolStep() {",
      "   const locations = tool?.locations ?? [];",
      "+  const fileChanges = tool?.fileChanges ?? [];",
      "+  const hasFileChanges = fileChanges.length > 0;",
    ].join("\n"),
    content: null,
    truncated: false,
    message: null,
  },
  {
    path: "apps/agentic-workbench/src/entities/agent-run/model/file-change-preview-with-an-extremely-long-file-name-for-layout-validation.ts",
    oldPath: null,
    kind: "added",
    status: "completed",
    binary: false,
    diff: null,
    content: "export const preview = true;\n",
    truncated: false,
    message: null,
  },
  {
    path: "assets/generated-preview.png",
    oldPath: null,
    kind: "modified",
    status: "unavailable",
    binary: true,
    diff: null,
    content: null,
    truncated: false,
    message: "Binary content cannot be displayed.",
  },
  {
    path: "docs/large-session-log.md",
    oldPath: null,
    kind: "modified",
    status: "completed",
    binary: false,
    diff: "--- a/docs/large-session-log.md\n+++ b/docs/large-session-log.md\n@@ -1 +1 @@\n-old\n+new",
    content: null,
    truncated: true,
    message: null,
  },
];

export const sampleWorktreeFiles: WorktreeFileEntry[] = [
  {
    name: "apps",
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/apps",
    relativePath: "apps",
    isDir: true,
    size: 0,
    modifiedMs: 1760000000000,
  },
  {
    name: "agentic-workbench",
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/apps/agentic-workbench",
    relativePath: "apps/agentic-workbench",
    isDir: true,
    size: 0,
    modifiedMs: 1760000000000,
  },
  {
    name: "src",
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/apps/agentic-workbench/src",
    relativePath: "apps/agentic-workbench/src",
    isDir: true,
    size: 0,
    modifiedMs: 1760000000000,
  },
  {
    name: "project-worktree-session-page.tsx",
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/apps/agentic-workbench/src/project-worktree-session-page.tsx",
    relativePath: "apps/agentic-workbench/src/project-worktree-session-page.tsx",
    isDir: false,
    size: 1820,
    modifiedMs: 1760000000000,
  },
  {
    name: "docs",
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/docs",
    relativePath: "docs",
    isDir: true,
    size: 0,
    modifiedMs: 1760000000000,
  },
  {
    name: "project-worktree-session-workspace-plan.md",
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/docs/project-worktree-session-workspace-plan.md",
    relativePath: "docs/project-worktree-session-workspace-plan.md",
    isDir: false,
    size: 4300,
    modifiedMs: 1760000000000,
  },
  {
    name: "README.md",
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/README.md",
    relativePath: "README.md",
    isDir: false,
    size: 720,
    modifiedMs: 1760000000000,
  },
];

export const sampleWorktreeTextFiles: Record<string, WorktreeTextFile> = {
  "README.md": {
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/README.md",
    relativePath: "README.md",
    content: "# Agentic Workbench\n\nStorybook에서 file preview를 확인하기 위한 샘플 파일입니다.\n",
    size: 720,
    truncated: false,
  },
  "docs/project-worktree-session-workspace-plan.md": {
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/docs/project-worktree-session-workspace-plan.md",
    relativePath: "docs/project-worktree-session-workspace-plan.md",
    content: "# ProjectWorktreeSessionPage Workspace 통합 계획\n\n- Git workspace\n- File tree\n- Markdown preview + annotation\n",
    size: 4300,
    truncated: false,
  },
  "apps/agentic-workbench/src/project-worktree-session-page.tsx": {
    path: "/Users/yoophi/project/worktrees/agentic-workbench/storybook/apps/agentic-workbench/src/project-worktree-session-page.tsx",
    relativePath: "apps/agentic-workbench/src/project-worktree-session-page.tsx",
    content: [
      "export function ProjectWorktreeSessionPage() {",
      "  return <main>Resizable agent workspace</main>;",
      "}",
    ].join("\n"),
    size: 1820,
    truncated: false,
  },
};

export const sampleGitHistory: GitCommitHistory = {
  commits: [
    {
      hash: "f6a4a7f7b7a3147d3eeb76e8c9bc90f7cdbf9011",
      message: "feat: add workspace file preview",
      author: "Yoophi",
      date: "2026-06-30T01:18:00Z",
    },
    {
      hash: "91fb001a3d52d237c1b8508f12f9ad1dd7593f00",
      message: "refine prompt actions and tool timeline",
      author: "Yoophi",
      date: "2026-06-29T23:30:00Z",
    },
  ],
  page: {
    offset: 0,
    limit: 100,
    totalCount: 2,
    hasMore: false,
  },
};

export const sampleGitGraph: GitCommitGraph = {
  commits: [
    {
      hash: "f6a4a7f7b7a3147d3eeb76e8c9bc90f7cdbf9011",
      shortHash: "f6a4a7f",
      parents: ["91fb001a3d52d237c1b8508f12f9ad1dd7593f00"],
      message: "feat: add workspace file preview",
      author: "Yoophi",
      date: "2026-06-30T01:18:00Z",
      isHead: true,
      isMerge: false,
    },
    {
      hash: "91fb001a3d52d237c1b8508f12f9ad1dd7593f00",
      shortHash: "91fb001",
      parents: [],
      message: "refine prompt actions and tool timeline",
      author: "Yoophi",
      date: "2026-06-29T23:30:00Z",
      isHead: false,
      isMerge: false,
    },
  ],
  refs: [
    {
      name: "feature/project-worktree-session-workspace",
      target: "f6a4a7f7b7a3147d3eeb76e8c9bc90f7cdbf9011",
      kind: "localBranch",
    },
  ],
  page: {
    offset: 0,
    limit: 300,
    totalCount: 2,
    hasMore: false,
  },
  layoutHints: {
    rowHeight: 32,
    maxInitialLanes: 10,
  },
};

export const sampleGitCommitDetail: GitCommitDetail = {
  ...sampleGitHistory.commits[0],
  files: [
    {
      path: "apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx",
      status: "M",
    },
    {
      path: "apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs",
      status: "A",
    },
  ],
};

export const sampleGitFileDiff: GitFileDiff = {
  commitHash: sampleGitCommitDetail.hash,
  path: sampleGitCommitDetail.files[0].path,
  content: [
    "diff --git a/worktree-workspace-panel.tsx b/worktree-workspace-panel.tsx",
    "@@ -1,3 +1,4 @@",
    "+export function WorktreeWorkspacePanel() {",
    "+  return <section />;",
    "+}",
  ].join("\n"),
  isBinary: false,
  isTruncated: false,
};

export const sampleGoal: ThreadGoal = {
  workingDirectory: "/Users/yoophi/project/agentic-workbench",
  objective: "Codex 스타일 /goal 기능을 완성하고 자동 continuation을 검증한다.",
  status: "paused",
  tokenBudget: 120000,
  tokensUsed: 42000,
  timeUsedSeconds: 180,
  createdAt: "2026-06-24T00:00:00Z",
  updatedAt: "2026-06-24T00:00:00Z",
};

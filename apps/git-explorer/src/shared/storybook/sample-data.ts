import type {
  AppInfo,
  GitBranch,
  GitCommitDetail,
  GitCommitGraph,
  GitCommitSummary,
  GitFileDiff,
  GitWorktree,
  Repository,
} from "@/entities/repository";

export const sampleAppInfo: AppInfo = {
  name: "Tauri Git Explorer",
  version: "0.1.0",
};

export const sampleRepositories: Repository[] = [
  {
    id: "repository-tauri-git-explorer",
    name: "tauri-git-explorer",
    path: "/Users/yoophi/project/tauri-git-explorer",
  },
  {
    id: "repository-acp-minimal-app",
    name: "acp-minimal-app",
    path: "/Users/yoophi/project/acp-minimal-app",
  },
  {
    id: "repository-spec-cat",
    name: "spec-cat",
    path: "/Users/yoophi/private/spec-cat",
  },
];

export const sampleWorktrees: GitWorktree[] = [
  {
    path: "/Users/yoophi/project/tauri-git-explorer",
    branch: "main",
    commit: "1d8d185a1f93a0d84f23b2579d40ca2de58424af",
    isBare: false,
    isMain: true,
  },
  {
    path: "/Users/yoophi/project/worktrees/tauri-git-explorer/git-graph",
    branch: "feat/git-graph",
    commit: "54c4a4e7e6f1a1d902dc7fa8927b4f50360b7790",
    isBare: false,
    isMain: false,
  },
];

export const sampleBranches: GitBranch[] = [
  {
    name: "main",
    fullName: "refs/heads/main",
    isRemote: false,
    isCurrent: true,
  },
  {
    name: "feat/git-graph",
    fullName: "refs/heads/feat/git-graph",
    isRemote: false,
    isCurrent: false,
    worktreePath: "/Users/yoophi/project/worktrees/tauri-git-explorer/git-graph",
  },
  {
    name: "feat/repository-actions",
    fullName: "refs/heads/feat/repository-actions",
    isRemote: false,
    isCurrent: false,
  },
  {
    name: "origin/main",
    fullName: "refs/remotes/origin/main",
    isRemote: true,
    isCurrent: false,
  },
  {
    name: "origin/feat/git-graph",
    fullName: "refs/remotes/origin/feat/git-graph",
    isRemote: true,
    isCurrent: false,
  },
];

export const sampleHistory: GitCommitSummary[] = [
  {
    hash: "1d8d185a1f93a0d84f23b2579d40ca2de58424af",
    message: "feat: improve commit file tree and diff views",
    author: "Yoophi",
    date: "2026-06-25 14:48:12 +0900",
  },
  {
    hash: "54c4a4e7e6f1a1d902dc7fa8927b4f50360b7790",
    message: "feat: render git graph with column layout",
    author: "Yoophi",
    date: "2026-06-25 13:54:01 +0900",
  },
  {
    hash: "9564445a6e7c5c9f6f0e0d8ab98275f599f3b6f7",
    message: "feat: rename and delete repositories",
    author: "Yoophi",
    date: "2026-06-25 12:40:30 +0900",
  },
  {
    hash: "7da55ca4b6bb2da2efb33dfd1c0f05b2e1e30118",
    message: "feat: refresh repository data on file changes",
    author: "Yoophi",
    date: "2026-06-25 11:10:08 +0900",
  },
];

export const sampleCommitGraph: GitCommitGraph = {
  commits: [
    {
      ...sampleHistory[0],
      shortHash: "1d8d185",
      parents: [sampleHistory[1].hash],
      isHead: true,
      isMerge: false,
    },
    {
      ...sampleHistory[1],
      shortHash: "54c4a4e",
      parents: [sampleHistory[2].hash],
      isHead: false,
      isMerge: false,
    },
    {
      ...sampleHistory[2],
      shortHash: "9564445",
      parents: [sampleHistory[3].hash],
      isHead: false,
      isMerge: false,
    },
    {
      ...sampleHistory[3],
      shortHash: "7da55ca",
      parents: [],
      isHead: false,
      isMerge: false,
    },
  ],
  refs: [
    {
      name: "main",
      target: sampleHistory[0].hash,
      kind: "localBranch",
    },
    {
      name: "origin/main",
      target: sampleHistory[1].hash,
      kind: "remoteBranch",
    },
    {
      name: "feat/git-graph",
      target: sampleHistory[1].hash,
      kind: "localBranch",
    },
    {
      name: "feat/repository-actions",
      target: sampleHistory[2].hash,
      kind: "localBranch",
    },
    {
      name: "origin/feat/git-graph",
      target: sampleHistory[1].hash,
      kind: "remoteBranch",
    },
    {
      name: "v0.1.0",
      target: sampleHistory[2].hash,
      kind: "tag",
    },
  ],
  page: {
    offset: 0,
    limit: 300,
    totalCount: 4,
    hasMore: false,
  },
  layoutHints: {
    rowHeight: 34,
    maxInitialLanes: 4,
  },
};

export const sampleCommitDetail: GitCommitDetail = {
  ...sampleHistory[0],
  files: [
    {
      path: "apps/desktop/src/widgets/changes-panel/ui/ChangesPanel.tsx",
      status: "M",
    },
    {
      path: "apps/desktop/src/shared/storybook/sample-data.ts",
      status: "A",
    },
    {
      path: "apps/desktop/src/stories/organisms.stories.tsx",
      status: "A",
    },
    {
      path: "packages/ui/src/components/table.tsx",
      status: "M",
    },
    {
      path: "docs/old-git-history-notes.md",
      status: "D",
    },
    {
      path: "apps/desktop/src/widgets/repository-sidebar/ui/RepositoryList.tsx",
      status: "R",
    },
  ],
};

export const sampleFileDiffs: Record<string, GitFileDiff> = {
  "apps/desktop/src/widgets/changes-panel/ui/ChangesPanel.tsx": {
    commitHash: sampleHistory[0].hash,
    path: "apps/desktop/src/widgets/changes-panel/ui/ChangesPanel.tsx",
    content: [
      "diff --git a/apps/desktop/src/widgets/changes-panel/ui/ChangesPanel.tsx b/apps/desktop/src/widgets/changes-panel/ui/ChangesPanel.tsx",
      "index 1111111..2222222 100644",
      "--- a/apps/desktop/src/widgets/changes-panel/ui/ChangesPanel.tsx",
      "+++ b/apps/desktop/src/widgets/changes-panel/ui/ChangesPanel.tsx",
      "@@ -163,7 +163,7 @@ export const GameFullscreenSlot = ({",
      "   return (",
      "     <div",
      "       id={slotId}",
      "-      className=\"h-full w-full overflow-hidden [&_iframe]:h-full [&_iframe]:w-full\"",
      "+      className=\"h-full w-full overflow-hidden [&>div]:h-full [&>div]:w-full [&_iframe]:h-full [&_iframe]:w-full\"",
      "     />",
      "   );",
    ].join("\n"),
    isBinary: false,
    isTruncated: false,
  },
  "docs/old-git-history-notes.md": {
    commitHash: sampleHistory[0].hash,
    path: "docs/old-git-history-notes.md",
    content: [
      "diff --git a/docs/old-git-history-notes.md b/docs/old-git-history-notes.md",
      "deleted file mode 100644",
      "--- a/docs/old-git-history-notes.md",
      "+++ /dev/null",
      "@@ -1,2 +0,0 @@",
      "-# Old notes",
      "-This content moved into the design document.",
    ].join("\n"),
    isBinary: false,
    isTruncated: false,
  },
  default: {
    commitHash: sampleHistory[0].hash,
    path: "apps/desktop/src/shared/storybook/sample-data.ts",
    content: [
      "diff --git a/apps/desktop/src/shared/storybook/sample-data.ts b/apps/desktop/src/shared/storybook/sample-data.ts",
      "new file mode 100644",
      "--- /dev/null",
      "+++ b/apps/desktop/src/shared/storybook/sample-data.ts",
      "@@ -0,0 +1,3 @@",
      "+export const sampleRepositories = [];",
      "+export const sampleHistory = [];",
      "+export const sampleBranches = [];",
    ].join("\n"),
    isBinary: false,
    isTruncated: false,
  },
};

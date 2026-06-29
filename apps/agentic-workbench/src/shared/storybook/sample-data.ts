import type {
  AgentDescriptor,
  ThreadGoal,
  WorktreeChange,
} from "@/entities/agent-run/model/types";
import type { GitBranch } from "@/entities/project/model/git-branch";
import type { GitRemote } from "@/entities/project/model/git-remote";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { GitWorktreeChanges } from "@/entities/project/model/git-worktree-changes";
import type { Project } from "@/entities/project/model/types";
import type { SavedPrompt } from "@/entities/saved-prompt/model/types";
import type {
  WorktreeFileEntry,
  WorktreeTextFile,
} from "@/entities/worktree-file/model/types";

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

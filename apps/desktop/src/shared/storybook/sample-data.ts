import type { AgentDescriptor, ThreadGoal } from "@/entities/agent-run/model/types";
import type { GitBranch } from "@/entities/project/model/git-branch";
import type { GitRemote } from "@/entities/project/model/git-remote";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { GitWorktreeChanges } from "@/entities/project/model/git-worktree-changes";
import type { Project } from "@/entities/project/model/types";
import type { SavedPrompt } from "@/entities/saved-prompt/model/types";

export const sampleProjects: Project[] = [
  {
    id: "project-acp",
    name: "ACP Minimal App",
    workingDirectory: "/Users/yoophi/project/acp-minimal-app",
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
    fetchUrl: "git@github.com:yoophi/acp-minimal-app.git",
    pushUrl: "git@github.com:yoophi/acp-minimal-app.git",
  },
];

export const sampleWorktrees: GitWorktree[] = [
  {
    path: "/Users/yoophi/project/acp-minimal-app",
    head: "a1b2c3d",
    branch: "main",
    status: "clean",
    canDelete: false,
  },
  {
    path: "/Users/yoophi/project/worktrees/acp-minimal-app/storybook",
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
  workingDirectory: "/Users/yoophi/project/worktrees/acp-minimal-app/storybook",
  stagedCount: 1,
  unstagedCount: 1,
  untrackedCount: 1,
  conflictedCount: 0,
  files: [
    {
      path: "apps/desktop/src/features/worktree-change-review/ui/worktree-changes-panel.tsx",
      stagedStatus: "M",
      unstagedStatus: null,
      group: "staged",
    },
    {
      path: "apps/desktop/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx",
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

export const sampleGoal: ThreadGoal = {
  workingDirectory: "/Users/yoophi/project/acp-minimal-app",
  objective: "Codex 스타일 /goal 기능을 완성하고 자동 continuation을 검증한다.",
  status: "paused",
  tokenBudget: 120000,
  tokensUsed: 42000,
  timeUsedSeconds: 180,
  createdAt: "2026-06-24T00:00:00Z",
  updatedAt: "2026-06-24T00:00:00Z",
};

import type { AgentDescriptor } from "@/entities/agent-run/model/types";
import type { GitBranch } from "@/entities/project/model/git-branch";
import type { GitRemote } from "@/entities/project/model/git-remote";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { Project } from "@/entities/project/model/types";

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

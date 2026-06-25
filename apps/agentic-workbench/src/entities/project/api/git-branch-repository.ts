import { invoke } from "@tauri-apps/api/core";

import type { GitBranch } from "@/entities/project/model/git-branch";

export async function listGitBranches(workingDirectory: string) {
  return invoke<GitBranch[]>("list_git_branches", { workingDirectory });
}

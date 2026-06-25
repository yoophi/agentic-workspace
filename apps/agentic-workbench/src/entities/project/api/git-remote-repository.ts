import { invoke } from "@tauri-apps/api/core";

import type { GitRemote } from "@/entities/project/model/git-remote";

export async function listGitRemotes(workingDirectory: string) {
  return invoke<GitRemote[]>("list_git_remotes", { workingDirectory });
}

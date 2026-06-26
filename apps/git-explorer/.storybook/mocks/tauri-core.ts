import {
  sampleAppInfo,
  sampleBranches,
  sampleCommitDetail,
  sampleCommitGraph,
  sampleFileDiffs,
  sampleHistory,
  sampleRepositories,
  sampleWorktrees,
} from "../../src/shared/storybook/sample-data";
import type { GitCommitGraph, GitCommitSummary, Repository } from "../../src/entities/repository";

let storybookRepositories: Repository[] = [...sampleRepositories];

type CommitRequest = {
  excludedRefs?: string[];
  includedRefs?: string[];
  maxCount?: number;
  offset?: number;
};

function shortRefName(fullName: string) {
  if (fullName.startsWith("refs/heads/")) {
    return fullName.slice("refs/heads/".length);
  }

  if (fullName.startsWith("refs/remotes/")) {
    return fullName.slice("refs/remotes/".length);
  }

  return fullName;
}

function commitHashesForRefs(refs: string[]) {
  const shortNames = refs.map(shortRefName);

  return new Set(
    sampleCommitGraph.refs
      .filter((ref) => shortNames.includes(shortRefName(ref.name)) || shortNames.includes(ref.name))
      .map((ref) => ref.target),
  );
}

function filteredHistory(request?: CommitRequest): GitCommitSummary[] {
  if (request?.includedRefs?.length) {
    const targetHashes = commitHashesForRefs(request.includedRefs);

    return sampleHistory.filter((commit) => targetHashes.has(commit.hash));
  }

  if (request?.excludedRefs?.length) {
    const targetHashes = commitHashesForRefs(request.excludedRefs);

    return sampleHistory.filter((commit) => !targetHashes.has(commit.hash));
  }

  return sampleHistory;
}

function filteredGraph(request?: CommitRequest): GitCommitGraph {
  const history = filteredHistory(request);
  const historyHashes = new Set(history.map((commit) => commit.hash));

  return {
    ...sampleCommitGraph,
    commits: sampleCommitGraph.commits.filter((commit) => historyHashes.has(commit.hash)),
    refs: sampleCommitGraph.refs.filter((ref) => historyHashes.has(ref.target)),
    page: {
      ...sampleCommitGraph.page,
      totalCount: history.length,
      hasMore: false,
    },
  };
}

export async function invoke<T>(command: string, args?: Record<string, unknown>) {
  switch (command) {
    case "app_info":
      return sampleAppInfo as T;
    case "list_repositories":
      return storybookRepositories as T;
    case "create_repository": {
      const request = args?.request as { path?: string } | undefined;
      const path = request?.path ?? "/Users/yoophi/project/tauri-git-explorer";
      const repository = {
        id: `storybook-repository-${storybookRepositories.length + 1}`,
        name: path.split("/").filter(Boolean).at(-1) ?? "repository",
        path,
      };
      storybookRepositories = [...storybookRepositories, repository];
      return repository as T;
    }
    case "rename_repository": {
      const request = args?.request as { repositoryId?: string; name?: string } | undefined;
      const repositoryId = request?.repositoryId ?? "";
      const renamedRepository = storybookRepositories.find(
        (repository) => repository.id === repositoryId,
      );

      if (!renamedRepository) {
        throw new Error(`Repository not found: ${repositoryId}`);
      }

      const nextRepository = {
        ...renamedRepository,
        name: request?.name ?? renamedRepository.name,
      };
      storybookRepositories = storybookRepositories.map((repository) =>
        repository.id === repositoryId ? nextRepository : repository,
      );
      return nextRepository as T;
    }
    case "delete_repository": {
      const request = args?.request as { repositoryId?: string } | undefined;
      storybookRepositories = storybookRepositories.filter(
        (repository) => repository.id !== request?.repositoryId,
      );
      return undefined as T;
    }
    case "list_worktrees":
      return sampleWorktrees as T;
    case "list_branches":
      return sampleBranches as T;
    case "list_history": {
      const request = args?.request as CommitRequest | undefined;
      const offset = request?.offset ?? 0;
      const limit = request?.maxCount ?? 100;
      const history = filteredHistory(request);
      const commits = history.slice(offset, offset + limit);

      return {
        commits,
        page: {
          offset,
          limit,
          totalCount: history.length,
          hasMore: offset + commits.length < history.length,
        },
      } as T;
    }
    case "get_commit_graph":
      return filteredGraph(args?.request as CommitRequest | undefined) as T;
    case "get_commit_detail":
      return sampleCommitDetail as T;
    case "get_file_diff": {
      const request = args?.request as { filePath?: string } | undefined;
      return (sampleFileDiffs[request?.filePath ?? ""] ?? sampleFileDiffs.default) as T;
    }
    case "start_repository_watchers":
    case "stop_repository_watchers":
      return undefined as T;
    default:
      throw new Error(`Unhandled Storybook Tauri command: ${command}`);
  }
}

import type {
  GitCommitGraph,
  GitCommitHistory,
  GitCommitSummary,
  GitGraphCommit,
} from "@yoophi/git-graph";

/**
 * cursor 기반 무한 스크롤의 페이지 파라미터(AW specs/007 research R8).
 * cursor는 마지막으로 받은 commit hash로, 백엔드가 이력 재작성을 감지하고
 * count/refs 재계산을 생략하는 데 쓴다.
 */
export type GitPageParam = { offset: number; cursor?: string };

export const initialGitPageParam: GitPageParam = { offset: 0 };

export function getNextGitPageParam(lastPage: {
  commits: Array<{ hash: string }>;
  page: { hasMore: boolean; offset: number };
}): GitPageParam | undefined {
  if (!lastPage.page.hasMore) {
    return undefined;
  }

  return {
    offset: lastPage.page.offset + lastPage.commits.length,
    cursor: lastPage.commits[lastPage.commits.length - 1]?.hash,
  };
}

/**
 * 무한 스크롤로 누적된 graph 페이지들을 하나의 그래프로 합친다.
 * 중복 커밋은 제거하고, page는 마지막 페이지 메타에 offset=0을 적용한다.
 */
export function combineGitCommitGraphPages(pages: GitCommitGraph[]) {
  const [firstPage] = pages;

  if (!firstPage) {
    return undefined;
  }

  const commits: GitGraphCommit[] = [];
  const commitHashes = new Set<string>();

  for (const page of pages) {
    for (const commit of page.commits) {
      if (commitHashes.has(commit.hash)) {
        continue;
      }

      commitHashes.add(commit.hash);
      commits.push(commit);
    }
  }

  const lastPage = pages[pages.length - 1] ?? firstPage;

  return {
    ...firstPage,
    commits,
    page: {
      ...lastPage.page,
      offset: 0,
      // totalCount와 refs는 첫 페이지에서만 계산된다(AW specs/007 R8).
      // 후속 페이지의 null이 첫 페이지 값을 덮어쓰지 않게 유지한다.
      totalCount: lastPage.page.totalCount ?? firstPage.page.totalCount,
    },
  };
}

/**
 * 무한 스크롤로 누적된 history 페이지들을 하나로 합친다. graph combiner와
 * 동일한 규칙: 중복 commit 제거, totalCount는 첫 페이지 값 유지(후속 페이지는
 * count를 생략해 null이다).
 */
export function combineGitCommitHistoryPages(pages: GitCommitHistory[]) {
  const [firstPage] = pages;

  if (!firstPage) {
    return undefined;
  }

  const commits: GitCommitSummary[] = [];
  const commitHashes = new Set<string>();

  for (const page of pages) {
    for (const commit of page.commits) {
      if (commitHashes.has(commit.hash)) {
        continue;
      }

      commitHashes.add(commit.hash);
      commits.push(commit);
    }
  }

  const lastPage = pages[pages.length - 1] ?? firstPage;

  return {
    commits,
    page: {
      ...lastPage.page,
      offset: 0,
      totalCount: firstPage.page.totalCount,
    },
  };
}

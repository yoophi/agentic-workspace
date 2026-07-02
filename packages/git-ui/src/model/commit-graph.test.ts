import { describe, expect, it } from "vitest";

import type { GitCommitGraph, GitGraphCommit } from "@yoophi/git-graph";

import {
  combineGitCommitGraphPages,
  combineGitCommitHistoryPages,
  getNextGitPageParam,
} from "./commit-graph";

function commit(hash: string): GitGraphCommit {
  return {
    hash,
    shortHash: hash.slice(0, 7),
    parents: [],
    message: `commit ${hash}`,
    author: "Test",
    date: "2026-07-02T00:00:00+09:00",
    isHead: false,
    isMerge: false,
  };
}

function page(overrides: Partial<GitCommitGraph["page"]>): GitCommitGraph["page"] {
  return {
    offset: 0,
    limit: 2,
    totalCount: null,
    hasMore: false,
    ...overrides,
  };
}

const layoutHints = { rowHeight: 32, maxInitialLanes: 10 };

describe("combineGitCommitGraphPages", () => {
  it("keeps first-page totalCount and refs when later pages omit them", () => {
    const firstPage: GitCommitGraph = {
      commits: [commit("aaa"), commit("bbb")],
      refs: [{ name: "main", target: "aaa", kind: "localBranch" }],
      page: page({ totalCount: 3, hasMore: true }),
      layoutHints,
    };
    const secondPage: GitCommitGraph = {
      commits: [commit("ccc")],
      refs: [],
      page: page({ offset: 2, totalCount: null, hasMore: false }),
      layoutHints,
    };

    const combined = combineGitCommitGraphPages([firstPage, secondPage]);

    expect(combined?.commits.map((entry) => entry.hash)).toEqual(["aaa", "bbb", "ccc"]);
    expect(combined?.page.totalCount).toBe(3);
    expect(combined?.page.hasMore).toBe(false);
    expect(combined?.refs).toHaveLength(1);
  });

  it("deduplicates commits across pages", () => {
    const firstPage: GitCommitGraph = {
      commits: [commit("aaa")],
      refs: [],
      page: page({ totalCount: 2, hasMore: true }),
      layoutHints,
    };
    const secondPage: GitCommitGraph = {
      commits: [commit("aaa"), commit("bbb")],
      refs: [],
      page: page({ offset: 1, hasMore: false }),
      layoutHints,
    };

    const combined = combineGitCommitGraphPages([firstPage, secondPage]);

    expect(combined?.commits.map((entry) => entry.hash)).toEqual(["aaa", "bbb"]);
  });
});

describe("getNextGitPageParam", () => {
  it("advances offset and carries the last commit hash as cursor", () => {
    const next = getNextGitPageParam({
      commits: [{ hash: "aaa" }, { hash: "bbb" }],
      page: { hasMore: true, offset: 2 },
    });

    expect(next).toEqual({ offset: 4, cursor: "bbb" });
  });

  it("stops when there are no more pages", () => {
    expect(
      getNextGitPageParam({ commits: [{ hash: "aaa" }], page: { hasMore: false, offset: 0 } }),
    ).toBeUndefined();
  });
});

describe("combineGitCommitHistoryPages", () => {
  it("keeps first-page totalCount and dedupes commits", () => {
    const combined = combineGitCommitHistoryPages([
      {
        commits: [
          { hash: "aaa", message: "a", author: "T", date: "2026-07-02" },
          { hash: "bbb", message: "b", author: "T", date: "2026-07-02" },
        ],
        page: { offset: 0, limit: 2, totalCount: 3, hasMore: true },
      },
      {
        commits: [
          { hash: "bbb", message: "b", author: "T", date: "2026-07-02" },
          { hash: "ccc", message: "c", author: "T", date: "2026-07-02" },
        ],
        page: { offset: 2, limit: 2, totalCount: null, hasMore: false },
      },
    ]);

    expect(combined?.commits.map((entry) => entry.hash)).toEqual(["aaa", "bbb", "ccc"]);
    expect(combined?.page.totalCount).toBe(3);
    expect(combined?.page.hasMore).toBe(false);
  });
});

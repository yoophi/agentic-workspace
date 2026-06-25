import { describe, expect, it } from "vitest";

import {
  gitStateRefreshIntervalMs,
  gitStateRefreshQueryOptions,
} from "./query-options";

describe("git state refresh query options", () => {
  it("polls frequently enough to surface external worktree branch changes", () => {
    expect(gitStateRefreshIntervalMs).toBe(3_000);
    expect(gitStateRefreshQueryOptions.refetchInterval).toBe(
      gitStateRefreshIntervalMs,
    );
  });

  it("keeps git state fresh after focus changes and while the window is inactive", () => {
    expect(gitStateRefreshQueryOptions).toMatchObject({
      refetchIntervalInBackground: true,
      refetchOnWindowFocus: true,
    });
  });
});

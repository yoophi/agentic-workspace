import { describe, expect, it } from "vitest";

import { AUTO_REFRESH_INTERVAL_MS, autoRefreshQueryOptions } from "./refresh-options";

describe("workspace auto refresh options", () => {
  it("uses a 3 second refresh interval", () => {
    expect(AUTO_REFRESH_INTERVAL_MS).toBe(3_000);
    expect(autoRefreshQueryOptions.refetchInterval).toBe(AUTO_REFRESH_INTERVAL_MS);
  });

  it("refreshes on focus and in background for desktop app windows", () => {
    expect(autoRefreshQueryOptions).toMatchObject({
      refetchIntervalInBackground: true,
      refetchOnWindowFocus: true,
    });
  });
});

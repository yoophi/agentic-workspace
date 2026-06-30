import { describe, expect, it } from "vitest";

import { AUTO_REFRESH_INTERVAL_MS, autoRefreshQueryOptions } from "./refresh-options";

describe("workspace auto refresh options", () => {
  it("uses a long fallback refresh interval", () => {
    expect(AUTO_REFRESH_INTERVAL_MS).toBe(30_000);
    expect(autoRefreshQueryOptions.refetchInterval).toBe(AUTO_REFRESH_INTERVAL_MS);
  });

  it("refreshes on focus without background polling", () => {
    expect(autoRefreshQueryOptions).toMatchObject({
      refetchIntervalInBackground: false,
      refetchOnWindowFocus: true,
    });
  });
});

import { describe, expect, it } from "vitest";

import { toTimelineItem } from "@/entities/agent-run/model/format";

describe("run event formatting", () => {
  it("shows Ralph loop iteration number and status", () => {
    const item = toTimelineItem("run-1", {
      type: "ralphLoop",
      iteration: 2,
      maxIterations: 5,
      status: "completed",
    });

    expect(item.group).toBe("lifecycle");
    expect(item.title).toBe("Ralph loop 2/5");
    expect(item.body).toBe("iteration 2/5: completed");
    expect(item.tone).toBe("success");
  });
});

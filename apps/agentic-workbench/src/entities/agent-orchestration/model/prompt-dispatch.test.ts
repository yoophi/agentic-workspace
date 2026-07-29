import { describe, expect, it } from "vitest";

import {
  initialPromptDispatchState,
  promptDispatchReducer,
  summarizePromptDispatch,
} from "./prompt-dispatch";

describe("promptDispatchReducer", () => {
  it("keeps per-target partial failures instead of rolling back a batch", () => {
    let state = promptDispatchReducer(initialPromptDispatchState, {
      type: "queued",
      dispatchId: "dispatch-1",
      panelIds: ["a", "b"],
    });
    state = promptDispatchReducer(state, { type: "succeeded", panelId: "a" });
    state = promptDispatchReducer(state, {
      type: "failed",
      panelId: "b",
      error: "run unavailable",
    });
    expect(summarizePromptDispatch(state)).toEqual({
      total: 2,
      succeeded: 1,
      failed: 1,
      pending: 0,
    });
  });
});

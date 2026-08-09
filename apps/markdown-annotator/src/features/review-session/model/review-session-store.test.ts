import { beforeEach, describe, expect, it } from "vitest";
import { useReviewSessionStore } from "./review-session-store";

describe("review session store", () => {
  beforeEach(() => useReviewSessionStore.setState({ session: { sessionId: "s", revision: 0, documentPath: "a.md", decision: "draft", annotations: [] }, warning: null }));
  it("creates and removes grouped annotations", () => { useReviewSessionStore.getState().add({ type: "note", comment: "c", selectedTexts: ["a", "b"] }); const items = useReviewSessionStore.getState().session?.annotations ?? []; expect(items).toHaveLength(2); expect(items[0]?.groupId).toBe(items[1]?.groupId); useReviewSessionStore.getState().removeGroup(items[0]!.id); expect(useReviewSessionStore.getState().session?.annotations).toHaveLength(0); });
  it("requires confirmation before approving open changes", () => { useReviewSessionStore.getState().add({ type: "change-request", comment: "fix", selectedTexts: ["a"] }); expect(useReviewSessionStore.getState().setDecision("approved")).toBe(false); expect(useReviewSessionStore.getState().setDecision("approved", true)).toBe(true); });
});

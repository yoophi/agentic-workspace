import { describe, expect, it } from "vitest";
import type { ReviewSession } from "@/entities/review-session/model/types";
import { ReviewSessionPersistence } from "./review-session-persistence";
const value = (id: string, revision = 0): ReviewSession => ({ sessionId: id, revision, documentPath: `${id}.md`, decision: "draft", annotations: [] });
describe("ReviewSessionPersistence", () => {
  it("discards a stale hydrate after a document switch", async () => { const resolves: Array<(v: ReviewSession) => void> = []; const subject = new ReviewSessionPersistence({ load: () => new Promise((resolve) => resolves.push(resolve)), save: async (v) => v }); const a = subject.hydrate("a"); const b = subject.hydrate("b"); resolves[0](value("a")); resolves[1](value("b")); expect(await a).toBeNull(); expect((await b)?.sessionId).toBe("b"); });
  it("serializes autosaves per session", async () => { const revisions: number[] = []; const subject = new ReviewSessionPersistence({ load: async () => value("a", 2), save: async (v, expected) => { revisions.push(expected); return { ...v, revision: expected + 1 }; } }); await Promise.all([subject.save(value("a")), subject.save(value("a"))]); expect(revisions).toEqual([0, 1]); });
  it("reloads after a revision conflict", async () => { let loads = 0; const subject = new ReviewSessionPersistence({ load: async () => { loads++; return value("a", 4); }, save: async () => { throw new Error("REVISION_CONFLICT"); } }); expect((await subject.save(value("a"))).revision).toBe(4); expect(loads).toBe(1); });
});

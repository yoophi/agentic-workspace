import type { ReviewSession } from "@/entities/review-session/model/types";

export type SessionPersistenceApi = { load: (key: string) => Promise<ReviewSession>; save: (session: ReviewSession, expectedRevision: number) => Promise<ReviewSession> };

export class ReviewSessionPersistence {
  private queues = new Map<string, Promise<ReviewSession>>();
  private generation = 0;
  constructor(private readonly api: SessionPersistenceApi) {}
  async hydrate(key: string) { const generation = ++this.generation; const session = await this.api.load(key); return generation === this.generation ? session : null; }
  save(session: ReviewSession) {
    const prior = this.queues.get(session.sessionId) ?? Promise.resolve(session);
    const queued = prior.catch(() => session).then(async (latest) => {
      try { return await this.api.save({ ...session, revision: latest.revision }, latest.revision); }
      catch (error) { if (String(error).includes("REVISION_CONFLICT")) return this.api.load(session.sessionId); throw error; }
    });
    this.queues.set(session.sessionId, queued);
    return queued;
  }
}

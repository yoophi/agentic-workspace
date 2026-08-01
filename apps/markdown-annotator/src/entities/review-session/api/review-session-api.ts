import { invoke } from "@tauri-apps/api/core";
import type { ReviewAnnotation, ReviewSession } from "../model/types";

type WireSession = Omit<ReviewSession, "annotations" | "documentPath"> & { document: NonNullable<ReviewSession["document"]>; annotations: Array<Omit<ReviewAnnotation, "id" | "type" | "selectedText"> & { annotationId: string; annotationType: ReviewAnnotation["type"]; anchor: { selectedText: string; prefix: string; suffix: string; headingPath: string[]; blockId: string | null; startOffset: number | null; endOffset: number | null }; createdAt: string; updatedAt: string }> };

const fromWire = (session: WireSession): ReviewSession => ({ ...session, documentPath: session.document.relativePath, annotations: session.annotations.map(({ annotationId, annotationType, anchor, ...annotation }) => ({ ...annotation, id: annotationId, type: annotationType, selectedText: anchor.selectedText })) });
export const toReviewSessionWire = (session: ReviewSession): WireSession => {
  const now = new Date().toISOString();
  const document = session.document ?? { rootId: "standalone", relativePath: session.documentPath, fingerprint: "", byteLength: 0, modifiedAtMs: null };
  return { ...session, schemaVersion: session.schemaVersion ?? 1, createdAt: session.createdAt ?? now, updatedAt: session.updatedAt ?? now, document, annotations: session.annotations.map(({ id, type, selectedText, ...annotation }) => ({ ...annotation, annotationId: id, annotationType: type, anchor: { selectedText, prefix: "", suffix: "", headingPath: [], blockId: null, startOffset: null, endOffset: null }, createdAt: now, updatedAt: now })) };
};

export async function loadReviewSession(rootId: string, relativePath: string, fingerprint: string, byteLength = 0, modifiedAtMs: number | null = null) {
  return fromWire(await invoke<WireSession>("load_review_session", { rootId, relativePath, fingerprint, byteLength, modifiedAtMs }));
}
export async function saveReviewSession(session: ReviewSession, expectedRevision: number) {
  try { return fromWire(await invoke<WireSession>("save_review_session", { session: toReviewSessionWire(session), expectedRevision })); }
  catch (error) { if (String(error).includes("RevisionConflict")) throw new Error("REVISION_CONFLICT"); throw error; }
}

export type ReviewAnnotationType = "change-request" | "question" | "note" | "delete";
export type ReviewDecision = "draft" | "changes-requested" | "approved" | "stopped";
export type ReviewAnnotation = { id: string; groupId: string | null; type: ReviewAnnotationType; status: "open" | "resolved"; comment: string; selectedText: string; attachmentState: "attached" | "conflict" | "orphan" | "missing" };
export type ReviewDocumentIdentity = { rootId: string; relativePath: string; fingerprint: string; byteLength: number; modifiedAtMs: number | null };
export type ReviewSession = { sessionId: string; revision: number; documentPath: string; decision: ReviewDecision; annotations: ReviewAnnotation[]; document?: ReviewDocumentIdentity; schemaVersion?: number; createdAt?: string; updatedAt?: string };

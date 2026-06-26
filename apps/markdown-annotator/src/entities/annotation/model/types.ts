export type AnnotationType = "delete" | "question" | "change-request" | "note" | "approve";

export type AnnotationAnchor = {
  blockId: string;
  startOffset?: number;
  endOffset?: number;
  selectedText?: string;
  startLine?: number;
  endLine?: number;
};

export type AnnotationDraft = {
  id: string;
  groupId?: string;
  fileName: string;
  anchor: AnnotationAnchor;
  selectedText: string;
  comment: string;
  type: AnnotationType;
  createdAt: string;
};

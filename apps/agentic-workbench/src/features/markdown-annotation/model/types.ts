export type MarkdownBlockType =
  | "heading"
  | "paragraph"
  | "blockquote"
  | "list-item"
  | "code"
  | "table"
  | "hr";

export type MarkdownBlock = {
  id: string;
  type: MarkdownBlockType;
  content: string;
  rawContent: string;
  order: number;
  startLine: number;
  endLine: number;
  level?: number;
  language?: string;
};

export type AnnotationType = "note" | "change-request" | "delete";

export type AnnotationAnchor = {
  blockId: string;
  startLine: number;
  endLine: number;
  startOffset?: number;
  endOffset?: number;
  selectedText?: string;
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

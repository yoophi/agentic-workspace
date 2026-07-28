export type MarkdownBlockType =
  | "heading"
  | "paragraph"
  | "blockquote"
  | "list-item"
  | "code"
  | "table"
  | "hr";

export type MermaidDetectionReason = "language-marker" | "leading-declaration";

export type MermaidBlockMetadata = {
  detected: true;
  reason: MermaidDetectionReason;
  declaration: string;
  source: string;
};

export type MarkdownSourceRange = {
  startOffset: number;
  endOffset: number;
  startColumn: number;
  endColumn: number;
};

export type MarkdownBlock = {
  id: string;
  type: MarkdownBlockType;
  content: string;
  rawContent: string;
  order: number;
  startLine: number;
  endLine: number;
  sourceRange?: MarkdownSourceRange;
  parentId?: string;
  level?: number;
  language?: string;
  mermaid?: MermaidBlockMetadata;
  ordered?: boolean;
  orderedStart?: number;
  checked?: boolean;
};

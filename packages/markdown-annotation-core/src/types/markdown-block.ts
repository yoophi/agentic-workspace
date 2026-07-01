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
  mermaid?: MermaidBlockMetadata;
  ordered?: boolean;
  orderedStart?: number;
  checked?: boolean;
};

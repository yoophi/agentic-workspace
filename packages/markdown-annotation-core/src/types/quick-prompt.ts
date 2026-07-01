import type { AnnotationAnchor } from "./annotation";

export type QuickPromptScopeKind = "selection" | "block" | "document";

export type QuickPromptActionSurface = "selection-toolbar" | "block-toolbar" | "document-action";

export type QuickPromptLengthState = "complete" | "reduced";

export type QuickPromptDraftStatus =
  | "editing"
  | "ready"
  | "sending"
  | "sent"
  | "blocked"
  | "failed";

export type AgentTargetAvailability = "available" | "unavailable" | "busy";

export type QuickPromptSelectionOffset = {
  blockId: string;
  startOffset?: number;
  endOffset?: number;
};

export type QuickPromptScope = {
  kind: QuickPromptScopeKind;
  label: string;
  blockId?: string;
  startLine?: number;
  endLine?: number;
  selectionOffsets?: QuickPromptSelectionOffset[];
};

export type QuickPromptContext = {
  id: string;
  scope: QuickPromptScope;
  sourceLabel: string;
  documentPath?: string;
  documentRevisionLabel: string;
  content: string;
  originalLength: number;
  includedLength: number;
  lengthState: QuickPromptLengthState;
  reductionReason?: string;
};

export type PromptDraft = {
  id: string;
  context: QuickPromptContext;
  promptText: string;
  status: QuickPromptDraftStatus;
  blockedReason?: string;
  createdAt: string;
  sentAt?: string;
};

export type AgentTarget = {
  id: string;
  label: string;
  availability: AgentTargetAvailability;
  unavailableReason?: string;
  ownerScope?: string;
};

export type QuickPromptActionState = {
  scopeKind: QuickPromptScopeKind;
  surface: QuickPromptActionSurface;
  enabled: boolean;
  disabledReason?: string;
  tooltip: string;
  accessibleName: string;
};

export type QuickPromptBuildOptions = {
  id?: string;
  sourceLabel: string;
  documentPath?: string;
  documentRevisionLabel?: string;
  maxContentLength?: number;
};

export type SelectionQuickPromptInput = QuickPromptBuildOptions & {
  anchors: AnnotationAnchor[];
  selectedText: string;
};

export type BlockQuickPromptInput = QuickPromptBuildOptions & {
  block: {
    id: string;
    rawContent: string;
    content: string;
    startLine: number;
    endLine: number;
    type?: string;
  };
};

export type DocumentQuickPromptInput = QuickPromptBuildOptions & {
  markdownText: string;
};

export type FormatQuickPromptOptions = {
  promptText: string;
  context: QuickPromptContext;
};

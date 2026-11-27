import type {
  BlockQuickPromptInput,
  DocumentQuickPromptInput,
  QuickPromptActionState,
  QuickPromptBuildOptions,
  QuickPromptContext,
  QuickPromptScope,
  SelectionQuickPromptInput,
} from "../types/quick-prompt";

const DEFAULT_MAX_CONTENT_LENGTH = 24_000;

function defaultId() {
  return `quick-prompt-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function trimDocumentPath(documentPath?: string) {
  const trimmed = documentPath?.trim();
  return trimmed || undefined;
}

function createRevisionLabel(label?: string) {
  return label?.trim() || new Date().toISOString();
}

function clampContent(content: string, maxContentLength = DEFAULT_MAX_CONTENT_LENGTH) {
  const originalLength = content.length;
  if (originalLength <= maxContentLength) {
    return {
      content,
      includedLength: originalLength,
      lengthState: "complete" as const,
      originalLength,
      reductionReason: undefined,
    };
  }

  const visibleLength = Math.max(0, maxContentLength);
  return {
    content: content.slice(0, visibleLength),
    includedLength: visibleLength,
    lengthState: "reduced" as const,
    originalLength,
    reductionReason: `Content was truncated from ${originalLength} to ${visibleLength} characters.`,
  };
}

function makeContext(
  scope: QuickPromptScope,
  rawContent: string,
  options: QuickPromptBuildOptions,
): QuickPromptContext {
  const content = rawContent.trim();
  if (!content) {
    throw new Error("Quick prompt context requires non-empty Markdown content.");
  }

  const bounded = clampContent(content, options.maxContentLength);
  return {
    id: options.id ?? defaultId(),
    scope,
    sourceLabel: options.sourceLabel,
    documentPath: trimDocumentPath(options.documentPath),
    documentRevisionLabel: createRevisionLabel(options.documentRevisionLabel),
    content: bounded.content,
    originalLength: bounded.originalLength,
    includedLength: bounded.includedLength,
    lengthState: bounded.lengthState,
    reductionReason: bounded.reductionReason,
  };
}

export function buildSelectionQuickPromptContext(input: SelectionQuickPromptInput) {
  const anchors = input.anchors.filter((anchor) => (anchor.selectedText ?? input.selectedText).trim());
  const lines = anchors.flatMap((anchor) => [
    anchor.startLine,
    anchor.endLine ?? anchor.startLine,
  ]).filter((line): line is number => line !== undefined);
  const firstAnchor = anchors[0];

  return makeContext(
    {
      kind: "selection",
      label: "선택 영역",
      blockId: firstAnchor?.blockId,
      startLine: lines.length > 0 ? Math.min(...lines) : undefined,
      endLine: lines.length > 0 ? Math.max(...lines) : undefined,
      selectionOffsets: anchors.map((anchor) => ({
        blockId: anchor.blockId,
        startOffset: anchor.startOffset,
        endOffset: anchor.endOffset,
      })),
    },
    anchors.length > 0
      ? anchors.map((anchor) => anchor.selectedText ?? input.selectedText).join("\n")
      : input.selectedText,
    input,
  );
}

export function buildBlockQuickPromptContext(input: BlockQuickPromptInput) {
  const rawContent = input.block.rawContent || input.block.content;
  return makeContext(
    {
      kind: "block",
      label: "Markdown 블럭",
      blockId: input.block.id,
      startLine: input.block.startLine,
      endLine: input.block.endLine,
    },
    rawContent,
    input,
  );
}

export function buildDocumentQuickPromptContext(input: DocumentQuickPromptInput) {
  const lineCount = input.markdownText ? input.markdownText.split(/\r?\n/).length : 0;
  return makeContext(
    {
      kind: "document",
      label: "전체 문서",
      startLine: lineCount > 0 ? 1 : undefined,
      endLine: lineCount > 0 ? lineCount : undefined,
    },
    input.markdownText,
    input,
  );
}

export function createQuickPromptActionState({
  scopeKind,
  surface,
  enabled,
  disabledReason,
}: {
  scopeKind: QuickPromptActionState["scopeKind"];
  surface?: QuickPromptActionState["surface"];
  enabled: boolean;
  disabledReason?: string;
}): QuickPromptActionState {
  const labels: Record<QuickPromptActionState["scopeKind"], string> = {
    selection: "선택 영역 quick prompt",
    block: "Markdown 블럭 quick prompt",
    document: "전체 문서 quick prompt",
  };
  const label = labels[scopeKind];

  return {
    scopeKind,
    surface: surface ?? defaultSurfaceForScope(scopeKind),
    enabled,
    disabledReason: enabled ? undefined : disabledReason,
    tooltip: enabled ? label : disabledReason ?? `${label}를 사용할 수 없습니다.`,
    accessibleName: label,
  };
}

function defaultSurfaceForScope(scopeKind: QuickPromptActionState["scopeKind"]): QuickPromptActionState["surface"] {
  if (scopeKind === "selection") {
    return "selection-toolbar";
  }

  if (scopeKind === "block") {
    return "block-toolbar";
  }

  return "document-action";
}

export function isQuickPromptSendable({
  promptText,
  context,
  targetAvailable,
}: {
  promptText: string;
  context: QuickPromptContext | null;
  targetAvailable: boolean;
}) {
  return Boolean(promptText.trim() && context?.content.trim() && targetAvailable);
}

import type { FormatQuickPromptOptions, QuickPromptContext } from "../types/quick-prompt";

function formatLineRange(context: QuickPromptContext) {
  if (context.scope.kind === "document") {
    return context.scope.endLine ? `- 범위: 전체 문서 (${context.scope.endLine} lines)` : "- 범위: 전체 문서";
  }

  if (context.scope.startLine === undefined) {
    return null;
  }

  if (context.scope.endLine === undefined || context.scope.startLine === context.scope.endLine) {
    return `- 행: ${context.scope.startLine}`;
  }

  return `- 행 범위: ${context.scope.startLine}-${context.scope.endLine}`;
}

function formatLengthState(context: QuickPromptContext) {
  if (context.lengthState === "complete") {
    return `- 첨부 상태: complete (${context.includedLength} chars)`;
  }

  return [
    `- 첨부 상태: reduced (${context.includedLength}/${context.originalLength} chars)`,
    context.reductionReason ? `- 축약 이유: ${context.reductionReason}` : null,
  ].filter(Boolean).join("\n");
}

export function formatQuickPromptForAgent({ promptText, context }: FormatQuickPromptOptions) {
  const trimmedPrompt = promptText.trim();
  if (!trimmedPrompt) {
    throw new Error("Quick prompt text is required.");
  }

  return [
    "# Markdown Quick Prompt",
    "",
    "## 요청",
    trimmedPrompt,
    "",
    "## 첨부 컨텍스트",
    `- 문서: ${context.documentPath || context.sourceLabel}`,
    `- 범위 유형: ${context.scope.label}`,
    formatLineRange(context),
    `- 문서 상태: ${context.documentRevisionLabel}`,
    formatLengthState(context),
    "",
    "```markdown",
    context.content,
    "```",
  ].filter((line): line is string => line !== null).join("\n");
}

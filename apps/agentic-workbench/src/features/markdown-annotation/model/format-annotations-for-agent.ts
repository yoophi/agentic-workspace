import type { AnnotationDraft, MarkdownBlock } from "./types";

export function formatAnnotationsForAgent(
  fileName: string,
  annotations: AnnotationDraft[],
  blocks: MarkdownBlock[],
) {
  if (annotations.length === 0) {
    return [
      "# Markdown Annotations",
      "",
      `File: ${fileName}`,
      "",
      "아직 annotation이 없습니다.",
    ].join("\n");
  }

  const sorted = [...annotations].sort((left, right) => {
    const leftBlock = blocks.findIndex((block) => block.id === left.anchor.blockId);
    const rightBlock = blocks.findIndex((block) => block.id === right.anchor.blockId);
    return leftBlock - rightBlock;
  });

  return [
    "# Markdown Annotations",
    "",
    `File: ${fileName}`,
    "",
    `이 Markdown 문서에 ${sorted.length}개의 피드백이 있습니다:`,
    "",
    ...sorted.map((annotation, index) =>
      [
        `## ${index + 1}. [${annotation.type}] ${formatAnnotationTitle(annotation.type)}`,
        "",
        annotation.anchor.startLine === annotation.anchor.endLine
          ? `- 행: ${annotation.anchor.startLine}`
          : `- 행 범위: ${annotation.anchor.startLine}-${annotation.anchor.endLine}`,
        annotation.anchor.startOffset !== undefined && annotation.anchor.endOffset !== undefined
          ? `- 선택 offset: ${annotation.anchor.startOffset}-${annotation.anchor.endOffset}`
          : "- 선택 범위: 전체 블록",
        "- 원본 Markdown:",
        "```markdown",
        annotation.selectedText,
        "```",
        "",
        formatAnnotationInstruction(annotation),
      ].join("\n"),
    ),
    "---",
    "",
    "위 delete와 change-request annotation은 문서 수정 요청으로 처리하고, note annotation은 참고 정보로만 사용하세요.",
  ].join("\n");
}

function formatAnnotationTitle(type: AnnotationDraft["type"]) {
  if (type === "change-request") {
    return "변경 요청";
  }
  if (type === "delete") {
    return "삭제 요청";
  }
  return "참고 메모";
}

function formatAnnotationInstruction(annotation: AnnotationDraft) {
  if (annotation.type === "change-request") {
    return `- 변경 요청: ${annotation.comment}`;
  }
  if (annotation.type === "delete") {
    return annotation.comment ? `- 삭제 이유: ${annotation.comment}` : "- 삭제 요청: 선택 영역을 삭제하세요.";
  }
  return `- 참고 메모: ${annotation.comment}`;
}

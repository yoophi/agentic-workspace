import type { AnnotationDraft } from "@/entities/annotation";
import type { MarkdownBlock } from "@/entities/markdown-block";

export type AgentPromptGoal = "edit-document" | "review-reference" | "custom";

export type FormatAnnotationsOptions = {
  filePath?: string;
  goal?: AgentPromptGoal;
  instruction?: string;
};

const promptGoalLabels: Record<AgentPromptGoal, string> = {
  "edit-document": "실행 가능한 annotation을 문서 수정 요청으로 반영합니다.",
  "review-reference": "annotation을 검토 참고용 메모로 사용합니다. 명시 요청이 없으면 문서를 수정하지 않습니다.",
  custom: "아래 사용자 지침을 우선합니다.",
};

function formatLineRange(annotation: AnnotationDraft) {
  if (annotation.anchor.startLine === undefined) {
    return null;
  }

  if (
    annotation.anchor.endLine === undefined ||
    annotation.anchor.endLine === annotation.anchor.startLine
  ) {
    return `- 행: ${annotation.anchor.startLine}`;
  }

  return `- 행 범위: ${annotation.anchor.startLine}-${annotation.anchor.endLine}`;
}

function formatGroupLineRange(annotations: AnnotationDraft[]) {
  const lines = annotations.flatMap((annotation) => [
    annotation.anchor.startLine,
    annotation.anchor.endLine ?? annotation.anchor.startLine,
  ]);
  const numericLines = lines.filter((line): line is number => line !== undefined);

  if (numericLines.length === 0) {
    return null;
  }

  const startLine = Math.min(...numericLines);
  const endLine = Math.max(...numericLines);

  return startLine === endLine ? `- 행: ${startLine}` : `- 행 범위: ${startLine}-${endLine}`;
}

function groupAnnotations(annotations: AnnotationDraft[]) {
  const groups = new Map<string, AnnotationDraft[]>();

  annotations.forEach((annotation) => {
    const key = annotation.groupId ?? annotation.id;
    const group = groups.get(key) ?? [];
    group.push(annotation);
    groups.set(key, group);
  });

  return Array.from(groups.values());
}

function annotationBlock(annotation: AnnotationDraft, blocks: MarkdownBlock[]) {
  return blocks.find((block) => block.id === annotation.anchor.blockId);
}

function isFullBlockAnnotation(annotation: AnnotationDraft, blocks: MarkdownBlock[]) {
  const block = annotationBlock(annotation, blocks);
  return block !== undefined && annotation.anchor.startOffset === 0 && annotation.anchor.endOffset === block.content.length;
}

function formatContext(annotation: AnnotationDraft, blocks: MarkdownBlock[], selectedText: string) {
  const block = annotationBlock(annotation, blocks);
  if (!block || block.content === selectedText || selectedText.length > 16) {
    return null;
  }

  return `- 원문 문맥: "${block.rawContent}"`;
}

function formatRawMarkdownContext(annotation: AnnotationDraft, blocks: MarkdownBlock[]) {
  const block = annotationBlock(annotation, blocks);
  if (!block?.rawContent) {
    return [];
  }

  return ["- 원본 Markdown:", "```markdown", block.rawContent, "```"];
}

function formatInstructionLine(annotation: AnnotationDraft) {
  if (annotation.type === "note") {
    return `- 참고 메모: ${annotation.comment}`;
  }

  if (annotation.type === "change-request") {
    return `- 선택 영역을 다음 내용으로 교체: ${annotation.comment}`;
  }

  if (annotation.type === "question") {
    return `- 질문: ${annotation.comment}`;
  }

  if (annotation.type === "approve") {
    return `- 승인 메모: ${annotation.comment}`;
  }

  return annotation.comment && !annotation.comment.startsWith("Remove this") ? `- 삭제 이유: ${annotation.comment}` : null;
}

function formatUserInstruction(instruction?: string) {
  const trimmedInstruction = instruction?.trim();
  if (!trimmedInstruction) {
    return [];
  }

  return ["사용자 지침:", "```", trimmedInstruction, "```"];
}

export function formatAnnotationsForAgent(
  fileName: string,
  annotations: AnnotationDraft[],
  blocks: MarkdownBlock[] = [],
  options: FormatAnnotationsOptions = {},
) {
  const targetFile = options.filePath || fileName;
  const goal = options.goal ?? "edit-document";
  const instruction = options.instruction?.trim();

  if (annotations.length === 0) {
    return [
      "# Markdown Annotations",
      "",
      `File: ${targetFile}`,
      `목표: ${promptGoalLabels[goal]}`,
      ...formatUserInstruction(instruction),
      "",
      "아직 annotation이 없습니다.",
    ].join("\n");
  }

  const sortedGroups = groupAnnotations(annotations).sort((a, b) => {
    const firstA = a[0];
    const firstB = b[0];
    const blockA = blocks.findIndex((block) => block.id === firstA?.anchor.blockId);
    const blockB = blocks.findIndex((block) => block.id === firstB?.anchor.blockId);
    if (blockA !== blockB) return blockA - blockB;
    return (firstA?.anchor.startOffset ?? 0) - (firstB?.anchor.startOffset ?? 0);
  });

  return [
    "# Markdown Annotations",
    "",
    `File: ${targetFile}`,
    `목표: ${promptGoalLabels[goal]}`,
    ...formatUserInstruction(instruction),
    "",
    `이 Markdown 문서에 ${sortedGroups.length}개의 피드백이 있습니다:`,
    "",
    ...sortedGroups
      .map((group, index) => {
        const annotation = group[0];
        if (!annotation) {
          return null;
        }
        const selectedText = group.map((item) => item.selectedText).join("\n");

        if (annotation.type === "delete") {
          const fullBlock = group.length === 1 && isFullBlockAnnotation(annotation, blocks);
          const deleteSnippet = fullBlock
            ? annotationBlock(annotation, blocks)?.rawContent ?? selectedText
            : selectedText;

          return [
            `## ${index + 1}. [delete] ${fullBlock ? "블록 삭제" : "선택 영역 삭제"}`,
            "",
            group.length > 1 ? formatGroupLineRange(group) : formatLineRange(annotation),
            group.length === 1 ? formatContext(annotation, blocks, selectedText) : null,
            "```markdown",
            deleteSnippet,
            "```",
            "",
            formatInstructionLine(annotation),
          ]
            .filter(Boolean)
            .join("\n");
        }

        return [
          `## ${index + 1}. [${annotation.type}] ${
            annotation.type === "change-request" ? "선택 영역 변경" : "선택 영역 피드백"
          }`,
          "",
          group.length > 1 ? formatGroupLineRange(group) : formatLineRange(annotation),
          group.length === 1 ? formatContext(annotation, blocks, selectedText) : null,
          ...(group.length === 1 && annotation.type === "change-request"
            ? formatRawMarkdownContext(annotation, blocks)
            : []),
          group.length === 1 && annotation.anchor.startOffset !== undefined && annotation.anchor.endOffset !== undefined
            ? `- Offset: ${annotation.anchor.startOffset}-${annotation.anchor.endOffset}`
            : null,
          `- 선택 영역: "${selectedText}"`,
          formatInstructionLine(annotation),
        ]
          .filter(Boolean)
          .join("\n");
      })
      .filter((section): section is string => section !== null),
    "---",
    "",
    goal === "edit-document"
      ? "위 delete와 change-request annotation을 문서에 반영하세요. note annotation은 참고 정보로만 사용하세요."
      : "위 annotation을 사용자 지침에 맞게 처리하세요.",
  ].join("\n");
}

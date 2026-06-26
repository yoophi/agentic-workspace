import { describe, expect, it } from "vitest";

import type { AnnotationDraft } from "@/entities/annotation";
import type { MarkdownBlock } from "@/entities/markdown-block";
import { formatAnnotationsForAgent } from "./formatAnnotationsForAgent";

const blocks: MarkdownBlock[] = [
  {
    id: "block-0",
    order: 0,
    type: "paragraph",
    content: "Buy onions",
    rawContent: "- Buy onions",
    startLine: 10,
    endLine: 10,
  },
  {
    id: "block-1",
    order: 1,
    type: "paragraph",
    content: "Keep receipt",
    rawContent: "Keep receipt",
    startLine: 12,
    endLine: 12,
  },
];

function annotation(overrides: Partial<AnnotationDraft>): AnnotationDraft {
  return {
    id: "annotation-1",
    fileName: "shopping.md",
    anchor: {
      blockId: "block-0",
      startOffset: 4,
      endOffset: 10,
      selectedText: "onions",
      startLine: 10,
      endLine: 10,
    },
    selectedText: "onions",
    comment: "shallots",
    type: "change-request",
    createdAt: "2026-06-26T00:00:00.000Z",
    ...overrides,
  };
}

describe("formatAnnotationsForAgent", () => {
  it("formats an empty annotation prompt with goal and user instruction", () => {
    expect(
      formatAnnotationsForAgent("shopping.md", [], blocks, {
        goal: "review-reference",
        instruction: "검토만 하세요.",
      }),
    ).toBe(`# Markdown Annotations

File: shopping.md
목표: annotation을 검토 참고용 메모로 사용합니다. 명시 요청이 없으면 문서를 수정하지 않습니다.
사용자 지침:
\`\`\`
검토만 하세요.
\`\`\`

아직 annotation이 없습니다.`);
  });

  it("formats change-request annotations with raw markdown context and offsets", () => {
    const prompt = formatAnnotationsForAgent(
      "shopping.md",
      [annotation({})],
      blocks,
      {
        filePath: "/tmp/shopping.md",
        instruction: "수정 요청만 반영하세요.",
      },
    );

    expect(prompt).toContain("File: /tmp/shopping.md");
    expect(prompt).toContain("## 1. [change-request] 선택 영역 변경");
    expect(prompt).toContain("- 행: 10");
    expect(prompt).toContain("- 원본 Markdown:\n```markdown\n- Buy onions\n```");
    expect(prompt).toContain("- Offset: 4-10");
    expect(prompt).toContain('- 선택 영역: "onions"');
    expect(prompt).toContain("- 선택 영역을 다음 내용으로 교체: shallots");
    expect(prompt).toContain("위 delete와 change-request annotation을 문서에 반영하세요.");
  });

  it("groups multi-block annotations and formats the combined line range", () => {
    const prompt = formatAnnotationsForAgent(
      "shopping.md",
      [
        annotation({
          id: "annotation-1",
          groupId: "selection-1",
          selectedText: "onions",
          type: "note",
          comment: "첫 번째 메모",
        }),
        annotation({
          id: "annotation-2",
          groupId: "selection-1",
          anchor: {
            blockId: "block-1",
            startOffset: 0,
            endOffset: 4,
            selectedText: "Keep",
            startLine: 12,
            endLine: 12,
          },
          selectedText: "Keep",
          type: "note",
          comment: "두 번째 메모",
        }),
      ],
      blocks,
    );

    expect(prompt).toContain("이 Markdown 문서에 1개의 피드백이 있습니다:");
    expect(prompt).toContain("## 1. [note] 선택 영역 피드백");
    expect(prompt).toContain("- 행 범위: 10-12");
    expect(prompt).toContain('- 선택 영역: "onions\nKeep"');
    expect(prompt).toContain("- 참고 메모: 첫 번째 메모");
  });

  it("formats full-block delete annotations with raw markdown", () => {
    const prompt = formatAnnotationsForAgent(
      "shopping.md",
      [
        annotation({
          type: "delete",
          comment: "불필요합니다.",
          anchor: {
            blockId: "block-0",
            startOffset: 0,
            endOffset: "Buy onions".length,
            selectedText: "Buy onions",
            startLine: 10,
          },
          selectedText: "Buy onions",
        }),
      ],
      blocks,
    );

    expect(prompt).toContain("## 1. [delete] 블록 삭제");
    expect(prompt).toContain("```markdown\n- Buy onions\n```");
    expect(prompt).toContain("- 삭제 이유: 불필요합니다.");
  });
});

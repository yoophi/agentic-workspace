import type { Meta, StoryObj } from "@storybook/react-vite";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";
import { MarkdownViewer, MermaidExpandedView } from "@yoophi/markdown-annotation-react";
import { markdownViewerComponents } from "@/shared/ui/markdown-viewer-components";

const meta = {
  title: "Molecules/MarkdownViewer",
  component: MarkdownViewer,
  args: {
    components: markdownViewerComponents,
  },
  parameters: {
    layout: "padded",
  },
} satisfies Meta<typeof MarkdownViewer>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Basic: Story = {
  args: {
    blocks: parseMarkdownToBlocks(`# Markdown Viewer

- GFM 목록
- **강조 텍스트**

| 영역 | 역할 |
| --- | --- |
| Renderer | Markdown 렌더링 |
| Selection | 선택 anchor 생성 |
`),
  },
};

export const TaskListStates: Story = {
  args: {
    blocks: parseMarkdownToBlocks(`# Task List Preview

- [ ] 미완료 작업
- [x] 완료된 작업
- [X] 대문자 X로 완료된 작업
  - [ ] 중첩된 미완료 작업과 [관련 문서](./related.md)
  - [x] 중첩된 완료 작업과 \`inline code\`
- 일반 목록은 기존 bullet로 표시됩니다.
- [ ] 공백 없이 매우 긴 작업 설명도 아이콘과 본문 정렬을 유지하며 미리보기 영역을 밀어내지 않고 자연스럽게 여러 줄로 표시되어야 합니다.

# Second Chapter Without Tasks

이 chapter에는 task가 없으므로 요약을 표시하지 않습니다.

# Third Chapter

- [ ] 세 번째 chapter의 미완료 작업
- [x] 세 번째 chapter의 완료 작업

\`\`\`md
- [ ] 코드 블록 안의 문구는 task로 표시하지 않습니다.
\`\`\`
`),
  },
};

export const HtmlCommentBoundaries: Story = {
  args: {
    blocks: parseMarkdownToBlocks(`# HTML5 Comment Boundaries

표시되는 본문입니다. <!-- Preview에서 숨겨지는 한 줄 주석 --> 이어지는 본문입니다.

<!--
여러 줄 주석도 Preview에 표시되지 않습니다.
-->

\`<!-- inline code는 보존 -->\`

\`\`\`html
<!-- fenced code의 내용도 보존 -->
\`\`\`

<!-- 닫히지 않은 주석은 이후 문서를 숨기지 않습니다.
`),
  },
};

export const MermaidDiagram: Story = {
  args: {
    blocks: parseMarkdownToBlocks(`# Mermaid Diagram

\`\`\`mermaid
flowchart TD
  A[Open document] --> B[Render Mermaid diagram]
  B --> C[Continue annotation workflow]
\`\`\`

\`\`\`ts
const ordinaryCode = true;
\`\`\`
`),
  },
};

export const MermaidFailureFallback: Story = {
  args: {
    blocks: parseMarkdownToBlocks(`# Mermaid Failure

\`\`\`mermaid
flowchart TD
  A --> 
\`\`\`

The rest of the document remains readable.
`),
  },
};

export const LargeMermaidDiagram: Story = {
  args: {
    blocks: parseMarkdownToBlocks(`# Large Mermaid Diagram

\`\`\`mermaid
flowchart LR
  A[Start] --> B[Collect requirements]
  B --> C[Design shared core detection]
  C --> D[Render diagram]
  D --> E[Show fallback on failure]
  E --> F[Verify annotation workflow]
  F --> G[Validate auto reload]
  G --> H[Finish]
  H --> I[Review]
  I --> J[Ship]
\`\`\`
`),
  },
};

export const MermaidExpandedModal: Story = {
  args: {
    blocks: [],
  },
  render: () => (
    <div className="max-w-3xl">
      <MermaidExpandedView
        blockId="storybook-ma-expanded-mermaid"
        components={markdownViewerComponents}
        defaultExpanded
        source={[
          "flowchart LR",
          "  A[Open document] --> B[Render Mermaid diagram]",
          "  B --> C[Open expanded modal]",
          "  C --> D[Inspect large diagram]",
          "  D --> E[Return to annotation]",
        ].join("\n")}
      />
    </div>
  ),
};

const blockActionMarkdown = `# Block Actions

Hover this paragraph to show the delete and comment controls for the whole block.

> This quoted block can also be annotated as a complete block.

\`\`\`ts
const enabled = true;
\`\`\`
`;
const blockActionBlocks = parseMarkdownToBlocks(blockActionMarkdown);

export const BlockActions: Story = {
  args: {
    blocks: blockActionBlocks,
    deletedBlockIds: new Set(blockActionBlocks[1] ? [blockActionBlocks[1].id] : []),
    inlineAnnotationsByBlock: new Map(
      blockActionBlocks[1] && blockActionBlocks[2]
        ? [
            [
              blockActionBlocks[1].id,
              [
                {
                  id: "inline-delete-1",
                  comment: "Remove this selected text.",
                  startOffset: 6,
                  endOffset: 20,
                  type: "delete",
                },
              ],
            ],
            [
              blockActionBlocks[2].id,
              [
                {
                  id: "inline-note-1",
                  comment: "This selected range has a note.",
                  startOffset: 5,
                  endOffset: 16,
                  type: "note",
                },
              ],
            ],
          ]
        : [],
    ),
    noteAnnotationsByBlock: new Map(
      blockActionBlocks[2]
        ? [
            [
              blockActionBlocks[2].id,
              [
                {
                  id: "note-1",
                  comment: "This block has a note annotation.",
                },
              ],
            ],
          ]
        : [],
    ),
    onCancelInlineAnnotation: (annotationId) => {
      console.log("cancel inline annotation", annotationId);
    },
    onEditInlineAnnotation: (annotationId) => {
      console.log("edit inline annotation", annotationId);
    },
    onRequestBlockComment: (block) => {
      console.log("comment block", block.id);
    },
    onRequestBlockDelete: (block) => {
      console.log("delete block", block.id);
    },
  },
};

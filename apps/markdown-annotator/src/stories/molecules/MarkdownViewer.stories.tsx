import type { Meta, StoryObj } from "@storybook/react-vite";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";
import { MarkdownViewer } from "@yoophi/markdown-annotation-react";
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

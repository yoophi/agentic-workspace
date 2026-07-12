import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";

export const previewBoundaryBlocks = parseMarkdownToBlocks(`# Preview Fixture

Visible <!-- hidden note --> text with [[plan | 구현 계획]].

- [ ] Open task
- [x] Completed task

\`<!-- inline code -->\`

\`\`\`md
<!-- fenced code -->
- [ ] code task
\`\`\``);

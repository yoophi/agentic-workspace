# Quickstart: Mermaid Chart Expanded Modal for Markdown Preview

## Prerequisites

- Install workspace dependencies with `pnpm install` if needed.
- Use the active feature artifacts under `specs/016-mermaid-modal-preview`.

## Validation Commands

Run shared package verification:

```bash
pnpm --filter @yoophi/markdown-annotation-react test
pnpm --filter @yoophi/markdown-annotation-react check-types
```

Run MA verification:

```bash
pnpm --filter @yoophi/markdown-annotator test
pnpm --filter @yoophi/markdown-annotator check-types
```

Run AW verification:

```bash
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/agentic-workbench check-types
```

Optional visual verification:

```bash
pnpm storybook:annotator
pnpm --filter @yoophi/agentic-workbench storybook
```

## Scenario 1: MA rendered diagram opens in modal

1. Open MA with a Markdown document containing a valid Mermaid fenced block.
2. Confirm the inline diagram renders.
3. Activate the diagram expand control.
4. Expected: a viewport-sized modal opens with the selected diagram.
5. Close the modal.
6. Expected: the original document position and annotation workflow remain usable.

## Scenario 2: AW markdown preview rendered diagram opens in modal

1. Open AW worktree session page.
2. Select a Markdown file containing a valid Mermaid fenced block in the markdown view panel preview.
3. Activate the diagram expand control.
4. Expected: a viewport-sized modal opens with the selected diagram.
5. Close the modal.
6. Expected: the markdown preview panel returns to the same context, and agent-run panel state is unchanged.

## Scenario 3: large diagram stays contained

1. Use a Mermaid fixture with a wide or tall diagram.
2. Open the expanded modal in MA and AW markdown preview.
3. Use fit, zoom in, and zoom out controls.
4. Expected: the diagram can be inspected with local modal scrolling, and the surrounding document or panel layout does not change.

## Scenario 4: failed and empty Mermaid blocks do not expose modal

1. Use fixtures with an empty Mermaid block and a syntactically invalid Mermaid block.
2. Open them in MA and AW markdown preview.
3. Expected: each block shows the existing fallback with source or failure reason.
4. Expected: no expand trigger is rendered for these fallback states.

## Scenario 5: ordinary code blocks remain ordinary

1. Use a document containing ordinary fenced code blocks next to Mermaid blocks.
2. Open the document in MA and AW markdown preview.
3. Expected: ordinary code blocks keep the existing code display and never show Mermaid expanded-view controls.

## Scenario 6: reload and file switch use latest source

1. Open a Markdown document with a Mermaid diagram in MA.
2. Change the file externally and let auto reload refresh the document.
3. Open the expanded modal.
4. Expected: the modal shows the latest diagram source.
5. Repeat the file switch path in AW worktree preview.
6. Expected: a newly selected file's diagrams open with the selected file content, not the prior file content.

## Expected Artifacts After Implementation

- Shared package tests for trigger visibility, fallback exclusion, zoom/fit body, and ordinary code preservation.
- MA tests or stories for Base UI dialog adapter behavior.
- AW tests or stories for Radix dialog adapter behavior in worktree preview and agent-run.
- No backend, persistence, or Tauri command changes.

## Validation Log

- 2026-07-07: `pnpm --filter @yoophi/markdown-annotation-react test` passed.
- 2026-07-07: `pnpm --filter @yoophi/markdown-annotation-react check-types` passed.
- 2026-07-07: `pnpm --filter @yoophi/markdown-annotator test` passed.
- 2026-07-07: `pnpm --filter @yoophi/markdown-annotator check-types` passed.
- 2026-07-07: `pnpm --filter @yoophi/agentic-workbench test` passed.
- 2026-07-07: `pnpm --filter @yoophi/agentic-workbench check-types` passed.
- 2026-07-07: Targeted app-to-app import scan found no direct imports between MA and AW.
- Deviations: None.

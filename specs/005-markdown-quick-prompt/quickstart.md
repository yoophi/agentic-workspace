# Quickstart: Markdown Quick Prompt Validation

## Prerequisites

- `pnpm install`이 완료되어 있어야 한다.
- Feature branch: `115-issue`
- Feature spec: `specs/005-markdown-quick-prompt/spec.md`

## Static Verification

```bash
pnpm --filter @yoophi/markdown-annotation-core check-types
pnpm --filter @yoophi/markdown-annotation-core test
pnpm --filter @yoophi/markdown-annotation-react check-types
pnpm --filter @yoophi/markdown-annotation-react test
pnpm --filter @yoophi/markdown-annotator check-types
pnpm --filter @yoophi/markdown-annotator test
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
```

Expected outcome:

- Core context builder and formatter tests pass.
- React viewer/action tests pass.
- MA and AW integrations type check and pass existing regression tests.

## Storybook Verification

```bash
pnpm storybook:annotator
```

Expected stories:

- Selection quick prompt state
- Block quick prompt action state
- Document quick prompt action state
- Empty selection/block/document state
- Unavailable agent target state
- Long context reduced state
- Keyboard focus and tooltip-visible state

Expected outcome:

- Lightning icon controls are visible in the correct surfaces.
- Tooltip and accessible labels communicate the action and disabled reason.
- Prompt composition shows context scope and length state before send.

## Manual MA Flow

1. Run `pnpm dev:annotator`.
2. Open the sample Markdown document.
3. Select a non-empty text range.
4. Trigger the selection quick prompt lightning action.
5. Enter a prompt such as `이 선택 영역을 더 명확하게 고쳐줘`.
6. Confirm the context preview says `선택 영역` and shows the selected text.
7. Confirm send is disabled or blocked with a clear unavailable-target reason when no agent target is connected.
8. Trigger a block quick prompt from a paragraph, list item, table, code block, and Mermaid block.
9. Trigger a document quick prompt.
10. Try empty document and empty/whitespace block fixtures.

Expected outcome:

- Valid contexts open the prompt composition UI.
- Invalid contexts do not send and show a reason.
- Existing annotation and selection actions still work.

## Manual AW Markdown Workspace Flow

1. Run `pnpm dev:workbench`.
2. Open a project worktree session with an active agent panel.
3. Open a Markdown file in the workspace Markdown view.
4. Trigger a block or document quick prompt.
5. Enter prompt text and send.

Expected outcome:

- The formatted prompt is delivered through the active agent prompt target.
- If the agent is busy, the app follows the existing queue/disabled policy.
- The receiving agent conversation contains the quick prompt request and attached context.

## Long Context Flow

1. Open a Markdown document with content longer than the configured quick prompt limit.
2. Trigger document quick prompt.
3. Inspect the context preview.

Expected outcome:

- The preview clearly indicates whether context is complete or reduced.
- The user can identify the included source scope before sending.

## Regression Flow

1. Add, edit, and delete an inline annotation.
2. Add and delete a block annotation.
3. Render a valid Mermaid diagram.
4. Render an invalid Mermaid diagram and inspect fallback.
5. Reload a changed document.

Expected outcome:

- Quick prompt controls do not hide, resize, or break existing annotation/rendering controls.
- Mermaid and ordinary code blocks keep their previous behavior.

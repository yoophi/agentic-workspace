# UI Contract: Mermaid Expanded View

## Consumers

- `apps/markdown-annotator`: MA document preview through shared `MarkdownViewer`.
- `apps/agentic-workbench`: worktree session markdown preview through shared `MarkdownViewer`.
- `apps/agentic-workbench`: agent-run Markdown output through existing streaming renderer and shared Mermaid components.

## Shared Package Contract

The shared React package must provide a Mermaid expanded-view capability that can be consumed without depending on Base UI, Radix, Tauri commands, app routes, or app-local persistence.

### `MermaidDiagram` behavior

- Renders inline loading, rendered, or failed state for a Mermaid source.
- Accepts optional render actions.
- Displays render actions only when the diagram reached `rendered` state.
- Keeps failed and empty states local to the block and exposes source/failure reason.
- Supports `fit` mode for constrained expanded viewports.

### Expanded view behavior

- Opens from a rendered Mermaid diagram trigger.
- Uses a viewport-sized dialog content area.
- Provides local overflow for diagrams larger than the viewport.
- Provides fit, zoom in, and zoom out actions with bounded zoom.
- Resets zoom to fit when opened.
- Closes without mutating host document, selection, file, session, or scroll state.

### Component injection behavior

Shared code may require app-provided primitives for:

- button rendering
- tooltip rendering
- dialog root/open state
- dialog trigger
- dialog content
- dialog title and description

The shared package must define the minimal TypeScript contract for those primitives. MA and AW must implement adapters in their own app source trees.

## Surface-Specific Requirements

### MA Markdown preview

- The expand trigger appears on rendered Mermaid diagrams in `MarkdownViewer`.
- Failed and empty Mermaid blocks keep existing fallback behavior with no expand trigger.
- Text selection, annotations, prompt-related actions, and auto reload remain available after opening and closing the modal.
- Storybook covers normal, large, and fallback Mermaid states.

### AW worktree markdown preview

- The expand trigger appears on rendered Mermaid diagrams in the worktree workspace preview.
- The modal is independent from agent-run panel state and worktree panel scroll state.
- File switching and preview reload update the source used by future modal openings.
- Existing annotation and preview workflows remain unchanged.

### AW agent-run

- The existing agent-run expanded Mermaid behavior remains available.
- Its trigger, zoom controls, fit behavior, and fallback exclusion stay consistent with the shared expanded-view contract.
- Streaming debounce or partial-output safeguards remain app-local where they are specific to agent-run output.

## Accessibility Contract

- Trigger has an accessible label equivalent to "Open Mermaid diagram in full screen".
- Modal has a title and screen-reader description.
- Close behavior is available through the app dialog primitive.
- Zoom controls have accessible labels.
- Focus handling follows the host dialog primitive.

## Negative Contract

- Do not expose modal actions for failed, empty, loading, or ordinary code blocks.
- Do not import from `apps/agentic-workbench` into `apps/markdown-annotator`, or the reverse.
- Do not add Tauri commands, file writes, stored preferences, or backend state.
- Do not weaken Mermaid's strict security rendering settings.

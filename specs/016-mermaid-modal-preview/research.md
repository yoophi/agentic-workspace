# Phase 0 Research: Mermaid Chart Expanded Modal for Markdown Preview

## Decision: expanded Mermaid viewer belongs in `packages/markdown-annotation-react`

**Rationale**: MA and AW worktree markdown preview already consume `MarkdownViewer` and `MermaidDiagram` from `@yoophi/markdown-annotation-react`. AW agent-run also imports the shared `MermaidDiagram` while keeping its own Markdown parsing. Moving the expanded-view contract into the shared package avoids app-to-app imports and keeps the behavior consistent where the UI contract has already converged.

**Alternatives considered**:

- Keep separate modal implementations in MA, AW preview, and AW agent-run. Rejected because it duplicates zoom, fit, fallback exclusion, and accessibility behavior across three surfaces.
- Move the AW agent-run component into MA. Rejected because direct app-to-app imports violate the monorepo boundary.
- Put the entire Markdown viewer into `apps/agentic-workbench`. Rejected because MA is already the other consumer and the shared package is the current reuse boundary.

## Decision: extend component injection for app-specific Dialog behavior

**Rationale**: MA uses Base UI shadcn primitives while AW uses Radix shadcn primitives. The shared package already hides Button and Tooltip differences behind `MarkdownViewerComponents`; the same pattern can expose a dialog shell/trigger contract without making shared code depend on either UI kit or app shell.

**Alternatives considered**:

- Add Radix or Base UI directly to the shared package. Rejected because it would force one app's dialog composition model onto the other.
- Implement modal entirely inside each app page. Rejected because `MermaidDiagram` owns render success/failure state and can reliably decide when expand actions should be visible.
- Use browser-native `<dialog>`. Rejected because existing app accessibility, styling, and close behavior are already standardized through shadcn dialog primitives.

## Decision: render expand actions only from the rendered state

**Rationale**: `MermaidDiagram` currently exposes `renderActions` only when state is `rendered`; fallback and loading states do not render actions. Keeping that behavior satisfies the requirement that failed, empty, or not-yet-rendered diagrams do not expose a modal trigger.

**Alternatives considered**:

- Show the trigger immediately while loading and let modal render its own fallback. Rejected because it creates a dead action for failed or empty diagrams and makes loading behavior inconsistent.
- Show the trigger for fallback so users can inspect source larger. Rejected because the feature is explicitly for rendered chart viewing; fallback source inspection already exists in the block.

## Decision: share zoom/fit body behavior, keep source rendering strict

**Rationale**: AW agent-run already solved the large-diagram problem by sizing an inner box by zoom percentage inside a local overflow container while rendering `MermaidDiagram fit`. This should become the shared expanded body so large diagrams can pan locally and fit to viewport without changing surrounding layout. Mermaid rendering keeps `securityLevel: "strict"`, `startOnLoad: false`, and existing source/failure handling.

**Alternatives considered**:

- CSS transform scaling. Rejected because existing agent-run implementation notes show transforms do not create scrollable overflow dimensions reliably at zoom levels above 100%.
- SVG-only post-processing. Rejected because it is more fragile across Mermaid diagram types and bypasses the current `MermaidDiagram` state model.
- No zoom controls, only overflow scroll. Rejected because the existing baseline experience includes fit/zoom controls and consistency is part of the user requirement.

## Decision: validate with package and consumer-app checks

**Rationale**: The shared package affects at least MA and AW, and the constitution requires atomic cross-app verification for `packages/*` changes. Tests should cover package behavior first, then app adapter/wiring behavior for Base UI and Radix consumers.

**Alternatives considered**:

- Only test package rendering. Rejected because app dialog adapters are the risk point for MA/AW differences.
- Only run full app tests. Rejected because shared package regressions should be caught at the smallest boundary.

## Decision: no backend, persistence, or documentation-by-default

**Rationale**: The feature adds a transient viewing interaction over existing rendered Markdown. It does not read new files, mutate documents, persist preferences, or touch Tauri commands. Project documentation is only needed if implementation changes the shared Markdown/Mermaid architecture in a way future contributors must follow.

**Alternatives considered**:

- Persist the last zoom level. Rejected as out of scope and not requested.
- Add backend commands for diagram export. Rejected as out of scope; the feature is inspection-only.

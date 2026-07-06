# Quickstart: ACP Tool List 기반 Prompt Command 자동완성 검증

## Prerequisites

- Repository dependencies are installed.
- Agentic Workbench can run in development mode.
- At least one agent/session configuration can expose one or more tool/command candidates, or Storybook mocks are available for UI validation.

## Static Checks

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
```

Expected outcome:

- Type checking passes.
- Prompt autocomplete helper tests pass.
- Prompt autocomplete UI/key handling tests pass.

If backend candidate lookup is implemented:

```bash
cargo test -p agentic-workbench
```

Expected outcome:

- Candidate normalization and session/run owner scope tests pass.
- Existing ACP runner/session tests continue to pass.

## Storybook Validation

```bash
pnpm --filter @yoophi/agentic-workbench storybook
```

Validate the prompt autocomplete states listed in [contracts/prompt-autocomplete-ui.md](./contracts/prompt-autocomplete-ui.md):

- loading candidates
- ready candidates
- many candidates with filtering
- no-match state
- empty source state
- long candidate name/description
- keyboard selection state

Expected outcome:

- Candidate list is readable and does not overlap prompt controls.
- Long text remains contained.
- Highlight state is visible.
- Empty/loading states do not disable typing.

## Manual End-to-End Scenario

1. Open Agentic Workbench on a worktree session.
2. Select an agent and ensure the prompt composer is in normal Prompt mode.
3. Focus the prompt textarea.
4. Type `$`.
5. Confirm candidate rows appear with names and descriptions.
6. Type part of a candidate name.
7. Confirm the list filters to relevant candidates.
8. Use `ArrowDown` or `ArrowUp` to change the highlighted candidate.
9. Press `Enter`.
10. Confirm the selected token is inserted into the prompt and the prompt is not submitted.
11. Type `$` or `/` again and press `Escape`.
12. Confirm the autocomplete closes without changing the prompt draft.

Expected outcome:

- `$` and `/` both open autocomplete.
- Candidate selection edits only the prompt draft.
- Prompt submission happens only through the normal submit action.

## Running-State Conflict Scenario

1. Start or attach to an active run where prompt sending/queueing is available.
2. Focus the prompt textarea and type a trigger token that opens autocomplete.
3. Press `Tab` while a candidate is highlighted.

Expected outcome:

- The highlighted candidate is inserted.
- The prompt is not queued.
- Existing queue shortcut still works when autocomplete is closed.

## Empty/Fallback Scenario

1. Open a session where no candidate source is ready or available.
2. Type `$unknown`.
3. Continue typing a normal prompt.
4. Submit the prompt through the normal prompt flow.

Expected outcome:

- No candidate state blocks typing.
- Prompt submission remains successful.

## Validation Results

Validated on 2026-07-07.

| Check | Command / Scope | Result |
|-------|------------------|--------|
| TypeScript typecheck | `pnpm --filter @yoophi/agentic-workbench check-types` | PASS |
| Frontend tests | `pnpm --filter @yoophi/agentic-workbench test` | PASS: 24 files, 127 tests |
| Rust backend tests | `cargo test -p agentic-workbench` | PASS: 163 lib tests, 0 bin/doc tests |
| Storybook static build | `pnpm --filter @yoophi/agentic-workbench build-storybook` | PASS |

Manual Tauri UI scenarios from this quickstart were not executed in a live desktop window during implementation. The same states are covered by Storybook static build and prompt autocomplete unit/static-render tests, but final product QA should still run the manual end-to-end and running-state conflict scenarios in the app.

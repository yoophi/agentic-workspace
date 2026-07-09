# Quickstart: Slash Command Keyboard Navigation

## Prerequisites

- From repository root: `/Users/yoophi/project/agentic-workspace`
- Node dependencies installed with pnpm.
- Current feature directory: `specs/025-slash-command-keyboard`

## Automated Validation

Run focused UI/model tests:

```sh
pnpm --filter agentic-workbench exec vitest run \
  src/features/agent-run/ui/prompt-command-autocomplete.test.tsx \
  src/features/agent-run/ui/agent-run-panel.test.tsx \
  src/entities/agent-run/model/prompt-autocomplete.test.ts
```

Run type checks:

```sh
pnpm --filter agentic-workbench check-types
```

Run full Agentic Workbench tests before completion:

```sh
pnpm --filter agentic-workbench test
```

## Manual Validation

1. Start the Agentic Workbench development app.
2. Open an Agent Run prompt where slash command autocomplete candidates are available.
3. Type a slash or skill-command trigger that shows many candidates.
4. Press `ArrowDown` repeatedly from the first candidate to the last candidate.
5. Confirm the highlighted candidate never leaves the visible suggestion container.
6. Press `ArrowUp` repeatedly from the last candidate back to the first candidate.
7. Confirm the highlighted candidate remains visible while the list scrolls.
8. Press `Enter` on a highlighted candidate.
9. Confirm that exactly one command is inserted into the prompt and the prompt editor keeps focus.
10. Repeat with `Tab` and confirm the same accepted autocomplete flow or next-step completion behavior.
11. Reopen autocomplete and press `Escape`.
12. Confirm autocomplete closes without unexpectedly changing prompt text.
13. Confirm long command names/descriptions do not overflow horizontally.
14. Confirm no selected command chips, multi-select labels, or command palette replacement UI appears.

## Expected Outcomes

- Keyboard-only navigation works without mouse interaction.
- Highlighted candidates remain visible in long scrollable lists.
- Prompt history arrow-key behavior still works when autocomplete is closed.
- Candidate application remains single-command insertion.
- Empty/loading/error/no-match states remain readable and non-crashing.

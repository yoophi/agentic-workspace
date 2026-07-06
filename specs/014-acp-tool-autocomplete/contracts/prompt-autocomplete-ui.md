# Contract: Prompt Autocomplete UI

## Purpose

Define the user-visible behavior for prompt command autocomplete in Agentic Workbench.

## Trigger Contract

- When the prompt textarea has focus and the current cursor token starts with `$` or `/`, the autocomplete surface opens.
- The typed prefix is preserved.
- The query is the text after the prefix until whitespace or line boundary.
- Removing the prefix or moving the cursor outside the trigger token closes the surface.

## Candidate Display Contract

- Each candidate row shows:
  - candidate name
  - short description when available
  - optional source/scope hint when needed to disambiguate duplicates
- Long names and descriptions must truncate or wrap without overlapping adjacent prompt controls.
- Loading, empty, no-match, and error states must not prevent typing in the textarea.

## Keyboard Contract

- When autocomplete is closed:
  - existing prompt submit, queue, and history navigation behavior remains unchanged.
- When autocomplete is open:
  - `ArrowDown` moves to the next candidate.
  - `ArrowUp` moves to the previous candidate.
  - `Enter` confirms the highlighted candidate and must not submit the prompt.
  - `Tab` confirms the highlighted candidate and must not queue the prompt.
  - `Escape` closes autocomplete and preserves the prompt draft.
  - Printable characters update the prompt draft and refilter candidates.

## Selection Contract

- Confirming a candidate replaces only the active trigger token range.
- Candidate selection updates prompt draft text and restores focus to the prompt textarea.
- Candidate selection does not submit the prompt.
- Candidate selection does not call an agent tool.
- Candidate selection does not approve permissions or alter permission mode.

## Pointer Contract

- Clicking a candidate confirms it with the same behavior as keyboard confirmation.
- Clicking outside the autocomplete surface closes it without changing prompt text.

## Accessibility Contract

- The autocomplete surface exposes listbox-like semantics.
- The highlighted candidate is announced or otherwise programmatically identifiable.
- Candidate rows have readable labels derived from name and description.

## Storybook States

Storybook must include representative states for:

- closed prompt composer
- prefix typed with loading candidates
- ready candidates with first item highlighted
- many candidates with filtering
- no matching candidates
- empty candidate source
- long candidate name and description
- keyboard-selected candidate

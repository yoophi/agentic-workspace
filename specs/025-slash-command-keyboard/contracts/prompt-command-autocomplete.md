# UI Contract: Prompt Command Autocomplete

## Purpose

Defines the user-visible and component-level contract for Agentic Workbench slash command autocomplete keyboard navigation.

## Scope

In scope:

- Existing prompt autocomplete surface.
- Single command candidate selection.
- Keyboard navigation and selection.
- Highlighted candidate visibility inside a scrollable list.
- Long text containment.

Out of scope:

- Multi-select chip UI.
- Standalone command palette replacement.
- Backend command candidate discovery changes.
- Persistence or settings changes.

## Prompt Textarea Keyboard Contract

| Key | When Autocomplete Is Open | Expected Result |
|-----|----------------------------|-----------------|
| `ArrowDown` | Ready candidates exist | Highlight moves to the next valid candidate and remains visible. |
| `ArrowUp` | Ready candidates exist | Highlight moves to the previous valid candidate and remains visible. |
| `Enter` | Ready candidates exist | Highlighted candidate is applied to the prompt and autocomplete closes. |
| `Tab` | Ready candidates exist | Highlighted candidate is applied or advances the same autocomplete flow without leaving an invalid state. |
| `Escape` | Any autocomplete status | Autocomplete closes or is suppressed without changing prompt text. |
| Other keys | Any autocomplete status | Existing prompt editing behavior continues unless the key changes the trigger/query. |

When autocomplete is closed, `ArrowUp` and `ArrowDown` must continue to support the existing prompt history behavior.

## List Rendering Contract

- The outer suggestion container exposes a listbox-style suggestion surface.
- Ready candidates expose option-style items with a clear selected/highlighted state.
- Loading, empty, no-match, and error states show a message and no selectable candidate.
- Candidate name, description, and source must fit inside the container without horizontal overflow.
- Highlighted item visibility must be maintained during keyboard navigation through long lists.

## Selection Contract

- Only one candidate can be applied at a time.
- Applying a candidate updates the current prompt command trigger range.
- Applying a candidate restores focus to the prompt editor.
- Selecting by pointer and selecting by keyboard must produce the same prompt insertion result.

## Regression Requirements

- A regression test must prove the autocomplete component exposes the structural hooks needed to keep highlighted items visible.
- A regression test must prove key handling preserves `Escape`, `Enter`, `Tab`, `ArrowUp`, and `ArrowDown` behavior.
- A regression test must prove the implementation does not introduce multi-select chips or selected command accumulation.
- A regression test must prove long candidate text is constrained within the suggestion item.

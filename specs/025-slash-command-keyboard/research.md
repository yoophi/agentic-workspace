# Research: Slash Command Keyboard Navigation

## Decision: Keep the existing prompt autocomplete surface

**Rationale**: The feature is about prompt command autocomplete, not selecting multiple values. The current prompt flow already tracks trigger range, filters command candidates, and replaces the trigger with one selected command. Replacing this with a multi-select or command palette would change the user workflow, add scope, and risk breaking prompt insertion semantics.

**Alternatives considered**:

- **Adopt a multi-select component pattern**: Rejected because selected command chips/labels do not match the single command insertion flow.
- **Use a command palette as a separate overlay**: Rejected for this feature because it would move selection away from the prompt editing context.
- **Use `cmdk` as the core autocomplete**: Deferred. `cmdk` is useful for command palettes and standalone command menus, but the existing textarea must keep focus for prompt editing and trigger replacement.

## Decision: Keep keyboard ownership in the prompt textarea

**Rationale**: Users are editing prompt text when autocomplete opens. The textarea already receives `ArrowUp`, `ArrowDown`, `Enter`, `Tab`, and `Escape`. Handling autocomplete keys there avoids moving DOM focus into the list and preserves cursor/selection state for command insertion.

**Alternatives considered**:

- **Move focus into list options**: Rejected because it complicates prompt cursor preservation and can make text editing shortcuts feel broken.
- **Use roving tabindex**: Rejected for this feature because the active editing control should remain the textarea.

## Decision: Maintain listbox semantics with visible highlighted item guarantee

**Rationale**: The list already renders a listbox-like suggestion surface. The missing behavior is keeping the highlighted candidate visible as keyboard navigation changes. The list should track highlighted candidate elements and scroll the nearest container only when needed, so movement is predictable and does not jump unnecessarily.

**Alternatives considered**:

- **Manual scrollTop calculations**: Possible but more brittle with variable item heights and text wrapping.
- **No scroll management**: Rejected because it is the reported defect.
- **Always scroll selected item to top**: Rejected because it is visually jumpy and disorients users.

## Decision: Clamp highlight state whenever candidate count changes

**Rationale**: Filtering can shrink the candidate list while a later item is highlighted. Highlight state must remain valid so keyboard selection never points at a missing item.

**Alternatives considered**:

- **Reset to first item on every filter change**: Predictable but can feel disruptive when the current item remains valid.
- **Allow invalid highlighted index until next key press**: Rejected because `Enter` or `Tab` could behave inconsistently.

## Decision: Preserve prompt history navigation when autocomplete is closed

**Rationale**: `ArrowUp` and `ArrowDown` already navigate prompt history at editable boundaries. Autocomplete should intercept those keys only when suggestions are open, and history navigation should keep working otherwise.

**Alternatives considered**:

- **Always reserve arrow keys for autocomplete**: Rejected because it would regress existing prompt history navigation.

## Decision: Verify with focused tests plus manual browser checks

**Rationale**: Static rendering can verify roles, text containment classes, and structural contracts. Source-level or component tests can verify key routing and callback contracts. Actual scroll visibility is best manually verified in the running app or with browser automation because jsdom has limited layout and scroll behavior.

**Alternatives considered**:

- **Only manual validation**: Rejected because keyboard routing and state contracts should be regression-tested.
- **Full browser automation only**: Deferred unless implementation exposes stable test selectors and the app server workflow is already active.

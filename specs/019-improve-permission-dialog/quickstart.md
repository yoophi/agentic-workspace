# Quickstart: 긴 Permission 다이얼로그 레이아웃 개선

## Prerequisites

- repository dependencies installed
- AW frontend tests can run locally
- Storybook can be started for `@yoophi/agentic-workbench`

## Automated Verification

```bash
pnpm --dir apps/agentic-workbench check-types
pnpm --dir apps/agentic-workbench test
```

Expected outcomes:

- Permission display helper tests pass for long option labels, command-like labels, empty labels, JSON input, and long single-line input.
- Permission dialog render tests pass for long content, narrow layout class/structure, pending state, and original `optionId` submission.
- Existing agent run panel tests continue to pass after dialog extraction.

## Storybook Verification

Start Storybook:

```bash
pnpm --dir apps/agentic-workbench storybook
```

Required stories:

- `Organisms/PermissionRequestDialog/LongCommand`
- `Organisms/PermissionRequestDialog/LongJsonPayload`
- `Organisms/PermissionRequestDialog/LongApprovalLabels`
- `Organisms/PermissionRequestDialog/NarrowWindow`

Expected outcomes:

- In every story, the action region remains visible and separate from detail content.
- Long command/payload content is readable inside the bounded detail region.
- Button labels are concise and do not contain the full command.
- The full option text or approval scope remains available in the dialog.

## Manual Validation Scenarios

### Scenario 1: 긴 command

1. Trigger or mock a permission request with a command longer than 5,000 characters.
2. Open the permission dialog.
3. Inspect the command detail and action buttons.

Expected:

- Dialog remains inside the window.
- Command detail can be reviewed without pushing buttons offscreen.
- Approval and rejection controls are visible and clickable.

### Scenario 2: 긴 approval prefix

1. Trigger or mock a permission request whose approval option name includes a long command prefix.
2. Open the permission dialog.

Expected:

- Button labels use short summaries.
- The original approval option text is still available outside the button label.
- Selecting a button submits the original `optionId`.

### Scenario 3: 좁은 창

1. Resize AW or Storybook viewport to about 360px width.
2. Open the long JSON payload permission dialog state.

Expected:

- Detail content does not overlap action controls.
- Buttons wrap or stack in a usable layout.
- Keyboard focus can move through detail/action controls.

### Scenario 4: submit failure recovery

1. Mock permission response submission to fail.
2. Select an approval option.

Expected:

- Pending state clears after failure.
- The user can choose an option again.
- Layout remains stable during and after pending state.

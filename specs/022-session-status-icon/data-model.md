# Data Model: Session Status Icon

## Session Information Update

Represents a provider-originated update about a session's metadata. It is not user-facing message content.

**Fields**:
- `sessionUpdate`: discriminator. Relevant value: `session_info_update`.
- `threadStatus`: optional nested status metadata for the agent thread.
- `title`: optional session title metadata.
- `updatedAt`: optional session freshness metadata.

**Validation rules**:
- Updates with `sessionUpdate = session_info_update` must not produce timeline message items.
- Missing `threadStatus` is valid and must not clear a known status unless the implementation explicitly resets the visible session.
- Unrecognized fields are ignored for timeline rendering.

## Agent Thread Status

Represents the latest known activity state for the visible agent session.

**Fields**:
- `type`: `active`, `idle`, or `unknown`.
- `activeFlags`: optional provider metadata for active status; not required for display in this feature.
- `receivedAt`: optional local time when the status was observed.

**State transitions**:
- `unknown -> active`: when a recognized active update arrives.
- `unknown -> idle`: when a recognized idle update arrives.
- `active -> idle`: when an idle update arrives.
- `idle -> active`: when an active update arrives.
- `active/idle -> active/idle`: repeated same-value updates refresh latest status but do not add timeline entries.
- `active/idle -> unknown`: only when the visible run/session resets or an explicit unknown state is chosen by the implementation.

**Validation rules**:
- `active` and `idle` must render as visually distinct states.
- Unknown or missing status must not show raw payload content.

## Timeline Message

Represents user-facing run content in chronological order.

**Fields**:
- `id`: stable item identifier.
- `runId`: owning run.
- `group`: message/tool/lifecycle/error/raw grouping.
- `title`: display title.
- `body`: user-facing text.
- `event`: typed run event backing the item.

**Validation rules**:
- User messages, agent messages, tool events, lifecycle events, permission events, terminal events, diagnostics, and errors must continue to appear according to existing behavior.
- `session_info_update` must not add a new Timeline Message.
- Existing raw event behavior outside `session_info_update` remains unchanged.

## Run Panel State

Represents the state needed to render the active run panel.

**Fields impacted by this feature**:
- `items`: timeline items, unchanged by session info updates.
- `agentThreadStatus`: latest known agent status for the active run/session.
- `isAwaitingPromptResponse`: may be cleared when an idle status arrives, matching existing intent.
- `activeRunId`: run identity used to ignore updates from inactive runs.

**Validation rules**:
- Events from inactive runs do not update status or timeline.
- Idle status updates clear awaiting-prompt state without adding timeline items.
- Active status updates make active state visible without adding timeline items.

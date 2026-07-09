# Research: Session Status Icon

## Decision: Treat `session_info_update` as session metadata, not timeline content

**Rationale**: The issue describes `session_info_update` as metadata about the session, not a user-facing message. Keeping it out of the timeline directly satisfies the primary user problem and matches existing handling for usage events, which update state without creating timeline entries.

**Alternatives considered**:
- Keep raw event in the Raw filter only: still risks visible raw JSON depending on the selected filter and does not solve the polluted timeline for ordinary users.
- Convert it to a lifecycle row: avoids JSON but still adds repeated noise for status changes.
- Hide all raw events: too broad because other raw diagnostics may still be useful during debugging.

## Decision: Add a typed/session-status classification helper in agent-run model

**Rationale**: The existing UI already contains ad hoc `isIdleThreadStatusEvent` logic inside `agent-run-panel.tsx`. Moving this into app-local model code makes active, idle, unknown, metadata-only, and repeated update handling testable without rendering the whole panel.

**Alternatives considered**:
- Keep parsing inside the React component: fastest edit but hard to test and likely to grow more component-level event parsing.
- Add a backend mapper: unnecessary because the raw payload is already received by the frontend and no persistence or session ownership behavior changes.
- Add a cross-app package: premature because only Agentic Workbench consumes this event shape.

## Decision: Store latest agent thread status in run panel state

**Rationale**: The UI must render the latest known active/idle state even after multiple events. The run panel state already tracks active run id, awaiting response, usage context, and timeline items, so adding latest status there keeps related session state together.

**Alternatives considered**:
- Derive status by scanning timeline items: impossible once `session_info_update` stops creating timeline items.
- Store status in global app state: broader than needed because the feature concerns the currently visible run panel.
- Store status only in component local state: works for rendering but leaves core event behavior harder to test.

## Decision: Display status near the run/session header or agent identity area

**Rationale**: The status is session-level metadata, not a message. A compact icon/badge near existing run controls makes it visible without competing with conversation content or adding another timeline event.

**Alternatives considered**:
- Add status messages to the timeline: contradicts the goal of reducing metadata noise.
- Add a modal/detail view only: users would not see active/idle state at a glance.
- Put the status only in window title: useful later for issue #113, but insufficient for this feature's session-local UI requirement.

## Decision: Unknown and partial updates should be neutral and non-disruptive

**Rationale**: The observed payloads include both status and metadata-only updates. Future provider versions may add new status values. The safest user-facing behavior is to suppress raw JSON for all `session_info_update` events and only update the visual status when a recognized status is present.

**Alternatives considered**:
- Show unknown values as raw JSON: reintroduces the main bug.
- Treat missing status as idle: can mislead users when the update only changes title or updated time.
- Throw or log visible errors: creates noise for benign metadata updates.

## Decision: No backend, storage, or migration work

**Rationale**: This feature changes how newly received session metadata is classified and shown. It does not need filesystem access, persisted settings, Tauri command changes, or migration of old timeline content.

**Alternatives considered**:
- Migrate historical raw metadata entries: larger scope and not required by the spec; old persisted data behavior can be addressed separately if needed.
- Persist latest thread status across app restarts: useful only if sessions are restored with stale status, but the spec focuses on visible state from incoming updates.

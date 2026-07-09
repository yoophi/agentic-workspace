# Research: Session Lifecycle Status

## Decision: Represent session lifecycle transitions as concise status messages

**Rationale**: Users need visible confirmation for session start and idle transition, but raw session update payloads are noisy and were already suppressed in prior work. A concise lifecycle/status message fits the existing timeline/status model while keeping agent responses and prompt messages visually distinct.

**Alternatives considered**:

- **Only use the header status badge**: Rejected because a badge shows current state but does not leave a readable transition in the work timeline.
- **Show raw `session_info_update` payloads**: Rejected because this was the original noise problem solved by earlier issues.
- **Create a separate full session inspector panel**: Deferred because the feature asks for short lifecycle/status messages, not a new inspection surface.

## Decision: Keep header status badge and timeline/status messages separate

**Rationale**: The header badge is the best surface for current agent state. Timeline/status messages are useful for meaningful transitions such as session started and idle entered. Keeping these roles separate prevents duplication with command summary and avoids overloading the header.

**Alternatives considered**:

- **Move all lifecycle state to the header**: Rejected because users still need chronological context.
- **Move all status into timeline only**: Rejected because current active/idle state should remain visible without scrolling.

## Decision: Dedupe repeated status updates per run

**Rationale**: ACP/session updates may repeat the same status. Repeated identical messages would bury useful timeline content. Dedupe should be scoped by run id and meaningful status key, so a new run starts fresh and real transitions still appear.

**Alternatives considered**:

- **Global dedupe across all runs**: Rejected because it could hide valid status messages for new runs.
- **No dedupe**: Rejected because it violates the requirement to avoid timeline pollution.
- **Time-window dedupe only**: Deferred because state-transition dedupe is simpler and more deterministic.

## Decision: Preserve backend event contract

**Rationale**: Existing frontend code already parses `session_info_update`, reads thread status, updates title/freshness, and suppresses raw output. The missing behavior is frontend status messaging and dedupe. Backend changes would add risk without clear necessity.

**Alternatives considered**:

- **Add a new backend typed event**: Rejected for this feature because the existing event data is sufficient.

## Decision: Verify with model tests, source/UI regression tests, and Storybook

**Rationale**: Status transition and dedupe behavior is pure enough for deterministic model tests. UI/source tests can verify raw suppression and coexistence with command summary. Storybook can show the visual state for lifecycle/status messages without requiring a live agent.

**Alternatives considered**:

- **Only manual testing**: Rejected because repeated update dedupe is regression-prone.
- **Only Storybook**: Rejected because Storybook does not prove transition/dedupe logic.

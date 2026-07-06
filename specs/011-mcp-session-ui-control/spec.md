# Feature Specification: MCP Session Title Control

**Feature Branch**: `011-mcp-session-ui-control`

**Created**: 2026-07-06

**Status**: Draft

**Input**: User description: "서비스에서 mcp 서버 (http 방식) 를 제공하고, agent 실행시 mcp 서버를 내장하려고 합니다. 이번 구현에서 mcp 서비스가 제공하는 기능은 윈도우 타이틀 변경으로 한정합니다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Agent labels a Worktree Session window (Priority: P1)

agent가 현재 작업 의도나 진행 중인 목표를 짧은 제목으로 제안하면, 사용자는 열린 Worktree Session 윈도우 제목에서 해당 세션의 목적을 빠르게 파악할 수 있다.

**Why this priority**: 여러 Worktree Session이나 agent 실행을 동시에 다룰 때 사용자는 창 제목으로 작업 맥락을 구분한다. 제목 변경은 agent가 사용자에게 현재 세션의 의미를 알려주는 가장 작고 직접적인 제어 기능이다.

**Independent Test**: 열린 Worktree Session에서 연결된 agent가 제목 변경을 요청했을 때, 해당 세션 윈도우의 제목이 요청한 값으로 갱신되고 다른 세션 윈도우 제목은 바뀌지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** Worktree Session 윈도우가 열려 있고 해당 세션에서 agent가 실행 중이다, **When** agent가 읽기 가능한 새 제목을 요청한다, **Then** 해당 윈도우 제목은 요청한 제목으로 갱신된다.
2. **Given** 두 개 이상의 Worktree Session 윈도우가 열려 있다, **When** 한 agent가 자기 세션의 제목 변경을 요청한다, **Then** 해당 agent가 소유한 윈도우 제목만 변경되고 다른 윈도우 제목은 유지된다.
3. **Given** 제목 변경 요청이 비어 있거나 허용 길이를 초과한다, **When** agent가 제목 변경을 요청한다, **Then** 시스템은 요청을 거부하거나 읽기 가능한 범위로 정리하고 결과를 agent에게 알려준다.

### Edge Cases

- agent가 닫힌 세션, 존재하지 않는 세션, 또는 다른 윈도우의 세션 제목을 변경하려고 하면 요청은 적용되지 않는다.
- 인증되지 않았거나 세션 소유권을 증명하지 못한 요청은 윈도우 제목을 변경하지 않는다.
- 제목이 공백뿐이거나 제어 문자처럼 사용자에게 보이지 않는 문자만 포함하면 요청은 유효하지 않은 제목으로 처리된다.
- 제목이 지나치게 길면 시스템은 명확한 길이 규칙에 따라 거부하거나 잘라내며, 결과를 agent에게 알려준다.
- 제목 변경 요청이 빠르게 연속으로 들어오면 해당 세션에는 마지막 유효 제목이 반영된다.
- agent 실행이 끝난 뒤에도 윈도우가 열려 있는 경우, 더 이상 유효하지 않은 agent 제어 요청은 적용되지 않는다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide an HTTP-compatible MCP service that is available to an agent during an active Worktree Session run.
- **FR-002**: System MUST expose only a window-title-change capability through this MCP service for the initial implementation.
- **FR-003**: System MUST associate each title-change request with exactly one active agent run and one Worktree Session window.
- **FR-004**: System MUST verify session ownership before changing any window title.
- **FR-005**: System MUST update only the target Worktree Session window title when an authorized agent submits a valid title.
- **FR-006**: System MUST reject blank, unreadable, or invalid title values without changing the current title.
- **FR-007**: System MUST apply a documented maximum title length or equivalent readability rule before displaying a title.
- **FR-008**: System MUST return a clear success or failure result for every title-change request so the agent can explain the outcome.
- **FR-009**: System MUST avoid exposing file contents, worktree browsing, Git change display, file preview control, Markdown preview control, or any file-modifying capability through this initial MCP service.
- **FR-010**: System MUST preserve the existing agent conversation, permission state, and user navigation state while applying a title change.
- **FR-011**: System MUST record enough diagnostic information for failed title-change requests to troubleshoot authorization or validation failures without exposing sensitive prompt or file content.

### Key Entities

- **Agent Control Session**: The authorized relationship between one agent run and one Worktree Session window, used to decide whether a title-change request may affect that window.
- **Title Change Request**: A request from the connected agent containing the desired human-readable window title.
- **Title Change Result**: The success or failure outcome returned to the agent, including a concise reason when the title is rejected or the session is unavailable.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: Scope is limited to `apps/agentic-workbench` because Worktree Session windows and agent runs are app-specific. Cross-app sharing is intentionally avoided for this initial feature.
- **Frontend layering**: Window/session composition remains in `app` and `pages`; title-change handling for the Worktree Session belongs in `features` only if reusable interaction state is needed. Existing entities for agent runs and projects remain the source of session identity.
- **Backend boundary**: Session ownership and request validation are application concerns. MCP transport, agent launch integration, and window event delivery are infrastructure or inbound adapter concerns. Domain models must remain independent of Tauri, filesystem, and HTTP details.
- **Shared core vs UI**: Any shared logic should be limited to headless title validation and control-result modeling. No shared UI package is required.
- **Persistence and safety**: The feature must validate run/session owner scope before title changes. It must not persist file content, expose worktree files, modify files, or grant cross-session control.
- **Documentation and Storybook**: Add Korean project documentation under `docs/*.md` with a Mermaid flow diagram for the agent-to-window-title path. Storybook is not required unless reusable UI states are introduced.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 95% of valid title-change requests, the target Worktree Session window title updates within 1 second while the window is open.
- **SC-002**: 100% of unauthorized, cross-session, closed-session, or invalid-title requests fail without changing any window title.
- **SC-003**: In a review with at least three open Worktree Session windows, users can identify the renamed session's purpose from the visible title in under 5 seconds.
- **SC-004**: The MCP service exposes no user-facing capabilities beyond title change in the initial implementation, verified by the available capability list.
- **SC-005**: 100% of tested title-change outcomes return an agent-readable success or failure result.

## Assumptions

- The first implementation intentionally excludes unstaged change display, source file display, Markdown file display, workspace refresh, file editing, staging, committing, and permission approval.
- The agent-facing control channel uses an HTTP-compatible MCP connection because the user explicitly requested that integration style.
- A single agent run controls only the Worktree Session window that launched it, even when multiple session windows are open.
- The default window title remains available before the agent requests a custom title and after invalid requests are rejected.
- Title changes are runtime UI state; durable persistence of agent-provided titles is out of scope unless later requested.

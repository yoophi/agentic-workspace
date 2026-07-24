# UI Contract: SDD 워크플로 단계 표시 및 제어

## Scope

이 계약은 AW Worktree Session의 Speckit tab이 활성 SDD 기능, 4단계 진행 상태, 작업 제어, 승인/재실행 확인, unavailable pointer 초기 prompt를 표시하는 방식을 정의한다.

## Active feature pointer

- Speckit tab은 `.specify/feature.json`의 `feature_directory`를 조회한다.
- 검증된 pointer와 feature 목록 항목이 일치하면 해당 row는 `현재 작업 중` 상태를 텍스트로 표시하고 시각적으로 highlight한다.
- highlight는 색상만으로 의미를 전달하지 않으며, assistive technology가 활성 상태를 알 수 있는 label 또는 description을 제공한다.
- pointer가 없거나 invalid/stale이면 어떤 feature row도 active로 표시하지 않는다.
- pointer 상태는 loading, unavailable, error를 구분해 설명한다.

## Stage controls

- controls는 `Specify`, `Plan`, `Tasks`, `Implement`를 항상 순서대로 표시한다.
- 각 stage는 complete, current, pending, unavailable을 구분 가능한 아이콘/텍스트로 나타낸다.
- 실행 불가 action은 disabled이며 필요한 선행 산출물 또는 active pointer 문제를 설명한다.
- 현재 또는 완료 stage에는 action을 제공하되, re-run은 confirmation dialog를 거친다.
- Plan action 전에는 spec review confirmation, Tasks action 전에는 plan review confirmation을 표시한다.
- confirm action은 active feature path와 stage가 명확히 표시된 prompt request를 기존 agent run area의 `queue` delivery로 전달한다. 요청은 즉시 실행하지 않는다.

## Initial SDD prompt

- active pointer unavailable 상태에서 사용자가 SDD 시작 제어를 선택하면, controls는 feature pointer 설정 또는 `$speckit-specify` 시작을 안내하는 편집 가능한 prompt를 만든다.
- 해당 request는 agent run textarea에 `draft` delivery로 주입된다.
- draft 주입은 실행·queue·history 기록을 일으키지 않는다.
- 사용자는 textarea에서 수정, 취소 또는 기존 Send action으로 명시 전송할 수 있다.

## Refresh and failure behavior

- watcher 또는 refresh는 active pointer, Speckit feature list, task progress를 함께 최신화한다.
- refresh 중에는 마지막으로 검증된 active state를 새 결과로 오인하지 않게 loading/refresh indication을 표시한다.
- pointer parse/read failure는 panel-level 안내로 제한하며, Speckit 문서 탐색과 preview는 계속 가능하다.

## Tasks Kanban and Needed Tasks

- 선택 문서가 `tasks.md`이면 사용자는 Preview, Kanban, 작업 필요 보기를 전환할 수 있다.
- Kanban은 미완료와 완료 열을 표시하고, 진행할 작업만/완료된 작업만 filter를 제공한다.
- 작업 필요 보기는 미완료 checkbox가 있는 section만 표시한다.
- 해당 section에는 heading, 필요한 non-task context, 미완료 checkbox만 표시한다.
- 완료 checkbox, 완료-only section, 선택된 미완료 task와 무관한 markdown 영역은 작업 필요 보기에 표시하지 않는다.
- checkbox가 없거나 `tasks.md`가 아니면 보기는 명확한 empty state를 표시하고 기존 preview는 계속 사용 가능하다.

## Acceptance Mapping

- FR-001, FR-001a, FR-001b: Active feature pointer
- FR-002, FR-003, FR-007, FR-012: Stage controls and refresh
- FR-004, FR-005, FR-006, FR-010: Stage action request and delivery
- FR-008, FR-009: Confirmation dialogs
- FR-011: Refresh and failure behavior
- FR-014, FR-015: Initial SDD prompt

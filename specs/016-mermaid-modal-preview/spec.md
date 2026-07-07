# Feature Specification: Mermaid Chart Expanded Modal for Markdown Preview

**Feature Branch**: `016-mermaid-modal-preview`

**Created**: 2026-07-07

**Status**: Draft

**Input**: User description: "AW worktreesessionpage 내 agent-run 패널에 적용된 mermaid chart 를 modal 로 크게 보기 기능을 MA 내 mermaid chart 및 worktreesessionpage 내 markdown view pannel 내 markdown preview 영역에 적용해주세요"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - MA Mermaid diagrams open in a large modal (Priority: P1)

Markdown Annotator에서 Mermaid 다이어그램이 포함된 문서를 검토하는 사용자는 작은 문서 영역 안에서 보기 어려운 차트도 큰 modal로 열어 전체 구조를 확인할 수 있다.

**Why this priority**: 요청의 핵심 대상 중 하나가 MA 내 Mermaid chart이며, 긴 흐름도나 넓은 구조도를 문서 안에서만 보는 것은 검토 작업을 어렵게 만든다.

**Independent Test**: MA에서 유효한 Mermaid 다이어그램이 포함된 문서를 열고, 다이어그램의 확대 보기 동작을 실행한 뒤 큰 modal에서 다이어그램을 확인하고 닫을 수 있는지 검증한다.

**Acceptance Scenarios**:

1. **Given** MA 문서 preview에 렌더링된 Mermaid 다이어그램이 있다, **When** 사용자가 확대 보기 동작을 실행한다, **Then** 해당 다이어그램은 viewport에 맞춘 큰 modal에서 표시된다.
2. **Given** MA의 Mermaid modal이 열린 상태다, **When** 사용자가 닫기 동작을 실행한다, **Then** modal은 닫히고 사용자는 기존 문서 위치와 검토 흐름으로 돌아간다.
3. **Given** MA 문서에 여러 Mermaid 다이어그램이 있다, **When** 사용자가 특정 다이어그램의 확대 보기 동작을 실행한다, **Then** 선택한 다이어그램만 modal에 표시된다.

---

### User Story 2 - AW markdown preview diagrams open in a large modal (Priority: P2)

Agentic Workbench의 worktree session page에서 markdown view panel의 preview 영역을 보는 사용자는 agent-run 패널과 같은 방식으로 Mermaid 다이어그램을 큰 modal에서 확인할 수 있다.

**Why this priority**: AW 안에서 agent-run 출력과 markdown preview는 모두 agent 산출물을 검토하는 화면이므로, 같은 Mermaid 다이어그램에 대해 일관된 확대 보기 경험을 제공해야 한다.

**Independent Test**: AW worktree session page의 markdown preview 영역에서 Mermaid 다이어그램이 포함된 markdown 파일을 열고, 다이어그램을 modal로 크게 볼 수 있는지 검증한다.

**Acceptance Scenarios**:

1. **Given** AW worktree session page의 markdown preview 영역에 렌더링된 Mermaid 다이어그램이 있다, **When** 사용자가 확대 보기 동작을 실행한다, **Then** 다이어그램은 큰 modal에서 표시된다.
2. **Given** AW markdown preview modal이 열린 상태다, **When** 사용자가 modal을 닫는다, **Then** 사용자는 기존 preview 문서 위치와 panel 상태를 유지한 채 돌아간다.
3. **Given** agent-run 패널과 markdown preview 영역이 같은 화면에 있다, **When** 사용자가 각 영역의 Mermaid 다이어그램 확대 보기를 사용한다, **Then** 두 영역은 서로의 스크롤 위치, panel 상태, 표시 내용을 변경하지 않는다.

---

### User Story 3 - Large, failed, and ordinary blocks remain stable (Priority: P3)

사용자는 크기가 큰 다이어그램, 렌더링 실패 상태, 일반 코드 블록이 섞인 문서를 보더라도 기존 읽기, 주석, 선택, preview 작업이 깨지지 않는 상태로 다이어그램만 필요한 경우 크게 볼 수 있다.

**Why this priority**: 확대 보기 기능은 기존 Mermaid 렌더링과 Markdown 검토 기능 위에 추가되는 보조 동작이므로, 실패 상태와 기존 workflow를 보존해야 실제 작업에서 신뢰할 수 있다.

**Independent Test**: 큰 Mermaid 다이어그램, 렌더링 실패 Mermaid 블록, 일반 코드 블록, 일반 문단이 함께 있는 문서를 MA와 AW markdown preview에서 열고 확대 보기 노출 여부와 기존 상호작용 유지 여부를 확인한다.

**Acceptance Scenarios**:

1. **Given** viewport보다 넓거나 긴 Mermaid 다이어그램이 있다, **When** 사용자가 modal로 연다, **Then** modal 안에서 다이어그램 내용을 탐색할 수 있고 문서 레이아웃은 변경되지 않는다.
2. **Given** Mermaid 렌더링이 실패한 블록이 있다, **When** 사용자가 문서를 확인한다, **Then** 실패 상태에는 불필요한 확대 보기 동작이 노출되지 않고 기존 원본 코드 또는 실패 이유 확인 흐름이 유지된다.
3. **Given** 일반 fenced code block이 있다, **When** 사용자가 MA 또는 AW markdown preview를 확인한다, **Then** 해당 블록은 일반 코드 블록으로 유지되며 Mermaid modal 동작이 적용되지 않는다.
4. **Given** MA 문서에서 텍스트 선택 또는 주석 작업을 수행할 수 있는 상태다, **When** 사용자가 Mermaid modal을 열고 닫은 뒤 다시 문서를 조작한다, **Then** 기존 선택과 주석 workflow가 계속 동작한다.

### Edge Cases

- Mermaid 언어 표기나 Mermaid 시작 문자열 감지는 각 화면의 기존 Mermaid 렌더링 규칙을 따른다.
- 비어 있거나 렌더링되지 않은 Mermaid 블록에는 큰 modal 열기 동작을 제공하지 않는다.
- 하나의 문서에 여러 다이어그램이 있어도 modal은 사용자가 선택한 다이어그램의 내용만 표시한다.
- modal 안의 다이어그램이 viewport보다 큰 경우 사용자는 modal 내부에서 내용을 탐색할 수 있어야 하며, 바깥 문서나 panel 레이아웃은 밀리거나 겹치지 않아야 한다.
- modal을 닫은 뒤 MA 문서 preview, AW markdown preview, agent-run 패널의 기존 스크롤과 선택 가능한 상태가 유지되어야 한다.
- 문서 자동 갱신이나 파일 전환으로 Mermaid 다이어그램 내용이 바뀌면 이후 확대 보기에는 최신 다이어그램이 표시되어야 한다.
- MA 또는 AW markdown preview에 Mermaid 다이어그램이 없는 경우 확대 보기 UI가 불필요하게 표시되지 않아야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: MA 내 렌더링된 Mermaid 다이어그램은 사용자가 큰 modal로 열어 볼 수 있는 확대 보기 동작을 제공해야 한다.
- **FR-002**: AW worktree session page의 markdown view panel 내 markdown preview 영역에서 렌더링된 Mermaid 다이어그램은 사용자가 큰 modal로 열어 볼 수 있는 확대 보기 동작을 제공해야 한다.
- **FR-003**: Mermaid expanded modal은 현재 viewport에 맞는 큰 보기 영역을 제공하고, 다이어그램이 보기 영역보다 크면 modal 내부에서 내용을 탐색할 수 있어야 한다.
- **FR-004**: 사용자는 modal을 닫고 원래 문서 또는 panel context로 돌아갈 수 있어야 하며, 닫기 후 기존 스크롤 위치와 검토 흐름이 유지되어야 한다.
- **FR-005**: 하나의 문서 또는 화면에 여러 Mermaid 다이어그램이 있을 때 확대 보기 동작은 사용자가 선택한 다이어그램에만 적용되어야 한다.
- **FR-006**: 렌더링 실패, 비어 있는 Mermaid 블록, 또는 아직 다이어그램으로 표시되지 않은 블록에는 큰 modal 열기 동작을 제공하지 않아야 한다.
- **FR-007**: 일반 코드 블록과 Mermaid가 아닌 Markdown 콘텐츠는 기존 표시 및 상호작용 방식을 유지해야 한다.
- **FR-008**: MA의 기존 Markdown 주석, 텍스트 선택, prompt 관련 작업, 자동 reload 흐름은 Mermaid modal 추가 후에도 계속 동작해야 한다.
- **FR-009**: AW markdown preview의 기존 문서 표시, 파일 전환, preview scroll, annotation 관련 흐름은 Mermaid modal 추가 후에도 계속 동작해야 한다.
- **FR-010**: AW agent-run 패널에 이미 제공되는 Mermaid 큰 보기 경험과 MA 및 AW markdown preview의 경험은 사용자가 보기에 일관된 affordance, 열기/닫기 동작, 실패 제외 규칙을 가져야 한다.
- **FR-011**: Mermaid modal은 untrusted Markdown 또는 agent 산출물로부터 unsafe behavior를 도입하지 않아야 한다.
- **FR-012**: 이 기능은 MA Mermaid chart, AW markdown preview Mermaid chart, 기존 AW agent-run Mermaid modal 사이의 중복된 사용자 경험을 검토하고, 필요한 경우 직접 앱 간 import 없이 공유 가능한 boundary를 통해 재사용되어야 한다.
- **FR-013**: 기능 검증은 MA Mermaid modal 성공/닫기, AW markdown preview Mermaid modal 성공/닫기, 큰 다이어그램 탐색, 실패 Mermaid 블록 제외, 일반 코드 블록 보존, 기존 주석 또는 preview workflow 보존을 포함해야 한다.

### Key Entities

- **Mermaid Diagram View**: MA 또는 AW markdown preview에 렌더링된 Mermaid 다이어그램 표시 상태이며, modal로 확대 가능한 대상이다.
- **Mermaid Expanded Modal**: 선택한 Mermaid Diagram View를 viewport에 맞춘 큰 영역에서 보여주는 일시적 보기 상태이며, 닫기 후 원래 문서 context로 돌아간다.
- **Markdown Preview Context**: MA 문서 preview 또는 AW worktree session page의 markdown preview 영역이며, 문서 위치, 선택 가능 상태, reload 또는 파일 전환 상태를 포함한다.
- **Render Fallback State**: Mermaid 다이어그램을 렌더링할 수 없거나 비어 있을 때 표시되는 블록 단위 상태이며, 확대 보기 대상에서 제외된다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/markdown-annotator`와 `apps/agentic-workbench`이다. 두 앱 간 직접 import는 금지하며, 공통 Mermaid modal 경험을 재사용해야 한다면 `packages/*`의 공유 모듈을 통해야 한다.
- **Frontend layering**: MA와 AW의 화면 조립은 각 앱의 `pages` 또는 관련 `features` layer에 둔다. 다이어그램 view 상태, open/close 상호작용, 재사용 가능한 UI primitive는 기존 구조에 맞춰 `features`, `entities`, `shared`, 또는 `components/ui` 경계를 지켜 배치한다.
- **Backend boundary**: 이 기능은 기존 Markdown 표시 경험 위의 frontend 상호작용이며 신규 Tauri backend, persistence, filesystem 동작을 요구하지 않는다.
- **Shared core vs UI**: 우선 공유 가능한 Mermaid detection/render-state 또는 modal view contract를 확인하고, MA와 AW의 interaction 및 visual contract가 같을 때만 공유 UI를 도입한다. 앱 shell, route state, Tauri command에 의존하는 UI는 공유하지 않는다.
- **Persistence and safety**: 기능은 기존 문서와 agent 산출물 표시 범위 안에서 동작한다. modal 상태는 휘발성이며 파일, session, permission, exchange ownership을 변경하지 않는다. 렌더링된 Markdown/diagram content는 unsafe behavior를 도입하지 않아야 한다.
- **Documentation and Storybook**: 재사용 가능한 Mermaid modal UI 또는 primitive가 도입되면 Storybook에 success, large diagram, failure-excluded 상태를 등록한다. 공유 Markdown/Mermaid architecture가 변경되면 `docs/*.md`에 한국어 문서를 추가하거나 갱신한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: MA의 대표 Mermaid fixture에서 렌더링에 성공한 다이어그램 100%가 큰 modal로 열리고 닫힌 뒤 원래 문서 검토 context로 돌아온다.
- **SC-002**: AW worktree session page markdown preview의 대표 Mermaid fixture에서 렌더링에 성공한 다이어그램 100%가 큰 modal로 열리고 닫힌 뒤 원래 preview context로 돌아온다.
- **SC-003**: 큰 다이어그램 fixture 100%가 modal 내부 탐색으로 확인 가능하며, modal 바깥 문서 또는 panel 레이아웃을 변경하지 않는다.
- **SC-004**: 렌더링 실패 또는 비어 있는 Mermaid 블록 fixture 100%에는 확대 보기 동작이 노출되지 않고 기존 fallback 확인 흐름이 유지된다.
- **SC-005**: 일반 코드 블록 fixture 100%는 Mermaid modal 동작 없이 기존 코드 블록 표시 방식으로 유지된다.
- **SC-006**: MA의 기존 주석, 텍스트 선택, 자동 reload 검증과 AW markdown preview의 파일 전환 및 preview 검증이 regression 없이 통과한다.
- **SC-007**: 사용성 검토에서 AW agent-run, AW markdown preview, MA Mermaid chart의 열기/닫기 affordance가 일관되다고 판단되는 비율이 주요 검토자 기준 90% 이상이다.

## Assumptions

- AW agent-run 패널의 Mermaid chart 큰 modal 기능은 이미 기준 경험으로 존재하며, 이번 기능은 그 사용자 경험을 MA와 AW markdown preview에 맞춰 확장한다.
- MA와 AW markdown preview의 기존 Mermaid 렌더링 규칙은 유지한다. 이 기능은 새로운 Mermaid 문법 감지 범위를 확대하는 것이 아니라, 성공적으로 렌더링된 다이어그램을 크게 보는 동작을 추가한다.
- modal 상태는 저장하지 않는다. 파일 전환, 문서 reload, 앱 재방문 후 modal은 기본적으로 닫힌 상태다.
- 큰 modal은 diagram inspection을 위한 보기 기능이며 Mermaid source 편집, 다이어그램 export, zoom ratio 저장은 범위에 포함하지 않는다.
- 접근성 기본 기대사항은 기존 modal pattern을 따른다. 사용자는 pointer 조작과 keyboard 기반 닫기 흐름으로 modal에서 빠져나올 수 있어야 한다.

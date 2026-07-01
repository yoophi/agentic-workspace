# Feature Specification: Markdown Quick Prompt

**Feature Branch**: `115-issue`

**Created**: 2026-07-02

**Status**: Draft

**Input**: User description: "GitHub issue #115: Markdown annotator에 quick prompt 기능 추가. 선택 영역, Markdown 블럭, 또는 전체 문서 내용을 바탕으로 번개 아이콘 quick prompt를 실행하고, 해당 컨텍스트가 첨부된 프롬프트를 작성/수정한 뒤 agent에게 바로 전달할 수 있게 한다. 첨부 범위 식별, disabled/empty 상태, 기존 annotation 선택/블럭 식별 로직과의 충돌 방지, 긴 문서 첨부 정책, agent 응답 연결 방식, tooltip/keyboard 접근성이 필요하다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 선택 영역을 agent 질문으로 전환 (Priority: P1)

Markdown 문서를 검토하는 사용자는 문서 안의 특정 문장이나 구간을 선택한 뒤 번개 아이콘으로 quick prompt 작성 UI를 열고, 선택한 텍스트가 첨부된 상태에서 agent에게 질문이나 작업 요청을 보낼 수 있다.

**Why this priority**: 현재 사용자가 수동 복사/붙여넣기로 처리하는 가장 빈번한 문서 검토 흐름을 끊김 없이 대체하는 핵심 가치이다.

**Independent Test**: 문서 일부 텍스트를 선택한 뒤 quick prompt를 실행하고, 선택 범위가 첨부 컨텍스트로 표시되며 사용자가 작성한 프롬프트와 함께 agent 대상에 전달되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** Markdown 문서에서 텍스트 일부가 선택되어 있다, **When** 사용자가 선택 영역 quick prompt 번개 아이콘을 실행한다, **Then** prompt 작성 UI가 열리고 첨부 컨텍스트는 "선택 영역"으로 식별된다.
2. **Given** 선택 영역 quick prompt 작성 UI가 열려 있다, **When** 사용자가 프롬프트 내용을 작성하거나 수정한다, **Then** 첨부 컨텍스트는 유지되고 사용자의 프롬프트 본문만 편집된다.
3. **Given** 선택 영역과 프롬프트 본문이 준비되어 있다, **When** 사용자가 전송한다, **Then** 선택 텍스트가 포함된 요청이 활성 agent 대상에 전달된다.

---

### User Story 2 - Markdown 블럭 단위로 요청 작성 (Priority: P2)

문서 구조를 기준으로 검토하는 사용자는 특정 Markdown 블럭 옆의 번개 아이콘을 사용해 해당 블럭 전체를 첨부한 prompt를 만들고 agent에게 작업을 요청할 수 있다.

**Why this priority**: 블럭 단위는 선택 조작 없이 제목, 목록, 코드, 표 같은 의미 단위를 빠르게 agent 작업 컨텍스트로 만들 수 있어 문서 검토 효율을 높인다.

**Independent Test**: 서로 다른 종류의 Markdown 블럭에서 quick prompt를 실행하고, 각 블럭 내용과 블럭 식별 정보가 첨부 컨텍스트로 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 사용자가 Markdown 문서의 특정 블럭을 보고 있다, **When** 해당 블럭의 quick prompt 번개 아이콘을 실행한다, **Then** prompt 작성 UI가 열리고 첨부 컨텍스트는 "Markdown 블럭"과 블럭 위치/요약으로 식별된다.
2. **Given** 블럭에 Markdown 원문과 렌더링된 표시가 모두 존재한다, **When** quick prompt 컨텍스트가 구성된다, **Then** agent가 작업할 수 있는 충분한 블럭 원문이 첨부된다.
3. **Given** 사용자가 블럭 단위 prompt를 전송한다, **When** agent 대상이 요청을 받는다, **Then** 사용자는 어떤 블럭이 요청에 포함되었는지 확인할 수 있다.

---

### User Story 3 - 전체 문서에 대해 질문 (Priority: P3)

문서 전체의 구조, 누락, 일관성을 검토하려는 사용자는 전체 문서 quick prompt를 실행해 문서 내용 전체를 첨부하고 agent에게 요약, 리뷰, 수정 제안을 요청할 수 있다.

**Why this priority**: 전체 문서 검토는 선택/블럭보다 범위가 크지만, 리뷰와 정리 작업에서 높은 가치를 제공한다.

**Independent Test**: 문서 단위 quick prompt를 실행하고, 전체 문서 범위가 명확히 표시되며 길이 제한 상태에서도 사용자가 전송 전 첨부 내용을 이해할 수 있는지 확인한다.

**Acceptance Scenarios**:

1. **Given** Markdown 문서가 열려 있다, **When** 사용자가 문서 단위 quick prompt를 실행한다, **Then** prompt 작성 UI가 열리고 첨부 컨텍스트는 "전체 문서"로 식별된다.
2. **Given** 전체 문서가 긴 내용을 포함한다, **When** quick prompt 컨텍스트가 준비된다, **Then** 사용자는 첨부되는 범위와 축약 또는 제한 여부를 전송 전에 확인할 수 있다.
3. **Given** 문서 단위 prompt가 전송된다, **When** agent 대상이 요청을 받는다, **Then** 요청은 사용자가 작성한 프롬프트와 문서 컨텍스트를 함께 포함한다.

---

### User Story 4 - 사용 불가 상태와 접근성 유지 (Priority: P4)

사용자는 선택 영역 없음, 빈 블럭, 빈 문서, agent 대상 없음 같은 상태에서도 앱 오류 없이 명확한 disabled 또는 empty 상태를 보고 키보드와 보조 기술로 quick prompt 진입점을 사용할 수 있다.

**Why this priority**: quick prompt는 문서 검토 화면 곳곳에 추가되는 상호작용이므로 기존 annotation 흐름을 방해하지 않고 실패 상태를 안전하게 처리해야 한다.

**Independent Test**: 각 empty/disabled 조건과 키보드 포커스 상태에서 quick prompt 버튼, tooltip, prompt 작성 UI가 예상대로 동작하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 선택 영역이 없다, **When** 사용자가 선택 영역 quick prompt 진입점을 확인한다, **Then** 해당 액션은 비활성화되거나 선택이 필요하다는 상태를 표시한다.
2. **Given** 빈 블럭 또는 빈 문서가 열려 있다, **When** 사용자가 quick prompt를 실행하려 한다, **Then** 앱 오류 없이 빈 컨텍스트 상태가 표시되고 전송은 방지된다.
3. **Given** 사용자가 키보드로 quick prompt 버튼에 접근한다, **When** 포커스와 실행 키를 사용한다, **Then** tooltip/이름/상태가 이해 가능하고 마우스 없이 prompt 작성 UI를 열 수 있다.

### Edge Cases

- 선택 영역이 여러 블럭에 걸쳐 있을 때 첨부 컨텍스트는 선택된 텍스트와 관련 범위를 사용자가 이해할 수 있게 표시해야 한다.
- 선택 영역이 변경된 직후 quick prompt를 실행해도 이전 선택과 새 선택이 혼동되지 않아야 한다.
- 빈 문자열, 공백만 있는 블럭, 또는 공백만 있는 문서는 전송 가능한 컨텍스트로 취급하지 않아야 한다.
- 문서가 자동 reload되는 중 prompt 작성 UI가 열려 있으면 사용자는 첨부 컨텍스트가 전송 시점의 어떤 문서 상태에서 왔는지 식별할 수 있어야 한다.
- 긴 문서 또는 긴 블럭은 전송 전 축약/제한 상태를 표시하고, 사용자가 포함 범위를 오해하지 않도록 해야 한다.
- agent 대상이 없거나 요청을 받을 수 없는 상태이면 전송 버튼은 비활성화되거나 실패 사유를 사용자에게 알려야 한다.
- 기존 annotation 선택, 블럭 선택, inline annotation 작성, Mermaid/code block 표시 동작이 quick prompt 버튼 때문에 가려지거나 오작동하지 않아야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: markdown annotator MUST provide a quick prompt entry point represented by a lightning icon for supported context scopes.
- **FR-002**: Users MUST be able to create a quick prompt from the current text selection when the selection contains non-empty Markdown document content.
- **FR-003**: Users MUST be able to create a quick prompt from a single Markdown block when that block contains non-empty content.
- **FR-004**: Users MUST be able to create a quick prompt from the entire open Markdown document when the document contains non-empty content.
- **FR-005**: The prompt composition UI MUST show the attached context scope before sending, distinguishing at least selected text, Markdown block, and entire document.
- **FR-006**: The prompt composition UI MUST allow users to write and edit the prompt text before sending while keeping the attached context visible.
- **FR-007**: The system MUST send the user's prompt text together with the attached context to the active agent target selected for the current workspace/session.
- **FR-008**: The system MUST prevent sending when the prompt text is empty, the attached context is empty, or no available agent target can receive the request.
- **FR-009**: The system MUST provide disabled or empty states for missing selection, empty block, empty document, and unavailable agent target conditions.
- **FR-010**: The attached context display MUST identify the source scope and provide enough location or summary information for the user to confirm what will be sent.
- **FR-011**: When context exceeds the supported prompt size for a single request, the system MUST visibly apply a bounded policy such as truncation, user-confirmed reduction, or summarized attachment before sending.
- **FR-012**: Quick prompt controls MUST include accessible names, keyboard focus support, and hover/focus tooltip text.
- **FR-013**: Quick prompt controls MUST NOT interfere with existing annotation creation, annotation editing, text selection, block detection, Mermaid rendering, or code block display workflows.
- **FR-014**: Agent responses produced from quick prompt requests MUST remain associated with the agent conversation that received the request; writing responses back into document annotations is out of scope for this release unless explicitly selected by the user in a future feature.
- **FR-015**: The feature MUST include representative verification or Storybook examples for selection, block, document, disabled/empty, long-context, and accessibility-visible states.

### Key Entities

- **Quick Prompt Context**: The document-derived content attached to a prompt; includes scope type, display label, source location or summary, raw Markdown content, and length/limit status.
- **Prompt Draft**: User-editable prompt text plus immutable attached context preview before the request is sent.
- **Agent Target**: The active destination that can receive the prompt request for the current workspace/session; has availability and sendability state visible to the user.
- **Context Scope**: The supported range used to build a quick prompt: selected text, Markdown block, or entire document.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: Primary scope is `apps/markdown-annotator`. Shared Markdown context extraction or prompt formatting logic may belong under `packages/markdown-annotation-core` if it must also be used by `apps/agentic-workbench`; direct app-to-app imports are out of scope.
- **Frontend layering**: User actions and prompt composition belong in the markdown annotator `features` layer. Screen placement belongs in `pages`. Markdown document, block, selection, and prompt context models belong in `entities`. Reusable UI primitives belong in `shared` or existing generated UI components.
- **Backend boundary**: No new persistence is required for the core quick prompt experience. If agent delivery requires a desktop command boundary, command handling must delegate to application services and must not embed business rules or persistence decisions directly in commands.
- **Shared core vs UI**: Pure context-scope modeling, block/selection extraction, and prompt formatting should be shared before any shared UI is introduced. Quick prompt UI remains app-local unless another app needs the same interaction contract.
- **Persistence and safety**: The feature uses already-open Markdown document content and must preserve existing file/root safety behavior. Prompt delivery must respect the active session/workspace owner and must not silently send content to an unrelated agent target.
- **Documentation and Storybook**: Add Storybook coverage for reusable quick prompt controls or prompt composition states. Project documentation is not required unless planning introduces a new cross-app prompt delivery contract.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In representative verification, users can open a quick prompt from a selected text range and see the selected text identified as the attached context in 100% of tested valid selections.
- **SC-002**: In representative verification, users can open a quick prompt from a Markdown block and from the entire document, with the correct scope label shown in 100% of tested valid cases.
- **SC-003**: At least 90% of usability review participants or internal reviewers can identify what content will be sent before pressing send without reading implementation notes.
- **SC-004**: Empty selection, empty block, empty document, and unavailable agent target states produce no app error and prevent invalid send attempts in 100% of tested cases.
- **SC-005**: Existing annotation selection/editing and Markdown rendering regression checks pass after quick prompt controls are added.
- **SC-006**: Keyboard-only users can focus, understand, open, edit, and send a valid quick prompt in the primary flow without using a pointer.
- **SC-007**: Long-context cases clearly indicate whether content is complete or reduced before sending in 100% of tested long document/block fixtures.

## Assumptions

- The first release targets markdown annotator as the quick prompt entry point; broader workbench-native Markdown panes may reuse pure context logic later if needed.
- "바로 agent에게 전달" means sending to the active agent target associated with the current workspace/session after the user reviews and submits the prompt draft.
- Quick prompt responses stay in the receiving agent conversation for this release; automatically creating or updating document annotations from the response is out of scope.
- The attached context should use Markdown source content when available because agent tasks usually need editable document text rather than only rendered display text.
- Long-context handling may use truncation, explicit reduction, or summarization as long as the user sees the resulting scope before sending.

# Feature Specification: Speckit Files Panel

**Feature Branch**: `[021-speckit-files-panel]`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "GitHub issue #111 Speckit 파일 전용 패널 추가"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Speckit 기능 목록을 한 곳에서 탐색하기 (Priority: P1)

사용자는 worktree session 화면에서 현재 worktree의 Speckit 기능 폴더와 주요 산출물을 전용 패널에서 한눈에 확인할 수 있다. 파일 트리를 직접 열어 `specs/*` 구조를 찾아다니지 않아도 기능별 문서를 빠르게 파악할 수 있어야 한다.

**Why this priority**: 이 기능의 핵심 가치는 Speckit 기반 작업 흐름에서 관련 산출물을 찾는 시간을 줄이고, 현재 기능의 spec/plan/tasks 상태를 즉시 확인하게 하는 것이다.

**Independent Test**: Speckit 구조가 있는 worktree session에서 Speckit 패널을 열고, 기능 폴더 목록과 각 폴더의 주요 문서가 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 현재 worktree에 하나 이상의 Speckit 기능 폴더가 있는 상태, **When** 사용자가 worktree session에서 Speckit 패널을 열면, **Then** 기능 폴더 목록과 각 폴더에 존재하는 주요 Speckit 문서가 표시된다.
2. **Given** 기능 폴더에 `spec.md`, `plan.md`, `tasks.md`가 있는 상태, **When** 사용자가 해당 기능 항목을 확인하면, **Then** 세 문서가 서로 구분된 항목으로 표시된다.
3. **Given** 기능 폴더에 선택 문서만 일부 존재하는 상태, **When** Speckit 패널이 목록을 표시하면, **Then** 존재하는 문서만 표시되고 없는 문서 때문에 오류가 발생하지 않는다.

---

### User Story 2 - Speckit 문서를 빠르게 열어 검토하기 (Priority: P2)

사용자는 Speckit 패널에서 특정 기능의 문서를 선택해 기존 markdown 검토 흐름으로 내용을 확인할 수 있다. 기능 산출물 탐색과 문서 확인이 같은 worktree session 안에서 이어져야 한다.

**Why this priority**: 문서 목록만 보여주는 것은 작업 흐름을 완성하지 못한다. 사용자가 원하는 문서를 즉시 열 수 있어야 Speckit 작업 맥락 전환 비용이 줄어든다.

**Independent Test**: Speckit 패널에서 특정 기능의 `spec.md`, `plan.md`, `tasks.md`를 각각 선택하고, 선택한 문서 내용이 기존 markdown 확인 영역에서 열리는지 확인한다.

**Acceptance Scenarios**:

1. **Given** Speckit 패널에 기능 문서 목록이 표시된 상태, **When** 사용자가 `spec.md` 항목을 선택하면, **Then** 해당 문서 내용이 markdown 검토 흐름에서 열린다.
2. **Given** 사용자가 한 기능의 `plan.md`를 보고 있는 상태, **When** 같은 기능의 `tasks.md`를 선택하면, **Then** 표시 대상이 선택한 `tasks.md`로 전환된다.
3. **Given** 선택한 문서를 읽을 수 없는 상태, **When** 사용자가 문서를 열려고 하면, **Then** 사용자는 실패 이유와 다시 시도할 수 있는 상태를 확인한다.

---

### User Story 3 - tasks 진행 상태를 요약해서 파악하기 (Priority: P3)

사용자는 각 기능의 `tasks.md`가 있는 경우 완료/미완료 작업 수와 진행 상태를 목록에서 바로 확인할 수 있다. 구현 전후 상태를 빠르게 비교하고 다음 작업 대상을 정할 수 있어야 한다.

**Why this priority**: Speckit 작업의 실행 가능성은 `tasks.md` 진행 상태와 밀접하다. 요약이 없으면 사용자는 매번 파일을 열어 체크박스를 직접 세어야 한다.

**Independent Test**: 체크박스가 포함된 `tasks.md`를 가진 기능과 없는 기능을 함께 표시하고, 진행 요약이 올바르게 표시되거나 비어 있는 상태가 안전하게 처리되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** `tasks.md`에 완료와 미완료 체크박스가 함께 있는 상태, **When** Speckit 패널이 기능 목록을 표시하면, **Then** 완료 수, 전체 수, 미완료 수 또는 동등한 진행 요약이 표시된다.
2. **Given** `tasks.md`가 있지만 체크박스 작업이 없는 상태, **When** 기능 항목이 표시되면, **Then** 진행률을 오해하게 만드는 숫자 대신 작업 항목 없음 상태가 표시된다.
3. **Given** `tasks.md`가 없는 기능 폴더, **When** 기능 항목이 표시되면, **Then** tasks 진행 요약은 없음 상태로 표시되고 다른 문서 탐색은 계속 가능하다.

---

### User Story 4 - Speckit 구조가 없는 저장소에서도 안전하게 동작하기 (Priority: P4)

사용자는 Speckit 구조가 없는 worktree에서도 오류나 빈 깨진 화면 대신 명확한 빈 상태를 본다. 이 상태는 사용자가 현재 저장소에 표시할 Speckit 문서가 없음을 이해하게 해야 한다.

**Why this priority**: 모든 저장소가 Speckit을 사용하지 않는다. 빈 상태가 안전해야 worktree session의 기본 사용성을 해치지 않는다.

**Independent Test**: `specs` 디렉터리가 없는 worktree와 빈 `specs` 디렉터리를 가진 worktree에서 Speckit 패널을 열고, 오류 없이 빈 상태가 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 현재 worktree에 `specs` 디렉터리가 없는 상태, **When** 사용자가 Speckit 패널을 열면, **Then** 표시할 Speckit 기능이 없다는 빈 상태가 표시된다.
2. **Given** `specs` 디렉터리는 있지만 기능 문서가 없는 상태, **When** 패널이 목록을 갱신하면, **Then** 빈 상태가 유지되고 worktree session의 다른 패널 사용은 영향을 받지 않는다.

### Edge Cases

- `specs` 아래에 문서 파일 없이 하위 디렉터리만 있거나, 기능 폴더가 아닌 일반 디렉터리가 섞여 있어도 패널은 유효한 Speckit 산출물만 표시해야 한다.
- `contracts/*`와 `checklists/*`처럼 하위 폴더에 있는 문서는 기능 문서 아래에서 구분 가능해야 한다.
- 기능 폴더 수나 문서 수가 많아도 사용자는 목록을 탐색할 수 있어야 하며, 패널이 과도하게 느려지거나 session 화면을 막으면 안 된다.
- 문서 파일이 패널 표시 직후 삭제, 추가, 수정되어도 목록과 선택 상태는 기존 files/git/markdown 패널 갱신 경험과 일관되게 갱신되어야 한다.
- 읽을 수 없는 파일, 깨진 텍스트, 권한 부족 파일은 전체 패널을 실패시키지 않고 해당 항목의 오류로 제한해야 한다.
- 같은 이름의 문서가 여러 기능 폴더에 있어도 사용자는 어떤 기능의 문서인지 구분할 수 있어야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 worktree session의 패널 목록에서 Speckit 전용 패널을 사용자가 선택할 수 있게 해야 한다.
- **FR-002**: 시스템은 현재 worktree 기준 `specs` 영역의 기능 폴더를 탐색하고, 표시 가능한 Speckit 기능 목록으로 제공해야 한다.
- **FR-003**: 시스템은 각 기능 항목에서 존재하는 주요 Speckit 문서를 문서 유형별로 구분해 표시해야 한다.
- **FR-004**: 시스템은 `spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `quickstart.md`, `contracts` 문서, `checklists` 문서를 표시 대상으로 포함해야 한다.
- **FR-005**: 사용자는 Speckit 패널에서 문서 항목을 선택해 해당 문서를 기존 markdown 검토 흐름에서 열 수 있어야 한다.
- **FR-006**: 시스템은 선택한 문서가 어느 기능 폴더에 속하는지 사용자가 식별할 수 있게 표시해야 한다.
- **FR-007**: 시스템은 `tasks.md`가 있는 기능에 대해 완료 작업 수와 전체 작업 수를 기반으로 진행 요약을 제공해야 한다.
- **FR-008**: 시스템은 `tasks.md`가 없거나 체크박스 작업이 없는 경우 진행 요약을 오해 없이 비어 있는 상태로 표시해야 한다.
- **FR-009**: 시스템은 `specs` 구조가 없거나 표시할 Speckit 문서가 없는 worktree에서 오류 없이 빈 상태를 표시해야 한다.
- **FR-010**: 시스템은 Speckit 문서 추가, 수정, 삭제가 발생하면 기존 worktree session의 파일/문서 갱신 경험과 일관된 방식으로 목록 또는 상태 요약을 갱신해야 한다.
- **FR-011**: 시스템은 읽기 실패, 권한 부족, 지원하지 않는 텍스트 상태를 전체 패널 오류가 아니라 해당 문서 또는 기능 항목의 오류로 제한해야 한다.
- **FR-012**: 시스템은 기능 목록과 문서 목록이 긴 경우에도 사용자가 항목을 스캔하고 선택할 수 있는 안정적인 목록 상태를 제공해야 한다.
- **FR-013**: 시스템은 Speckit 패널의 기본, 빈 상태, 오류 상태, 긴 목록 상태, tasks 진행 요약 상태를 검증 가능한 시나리오로 확인할 수 있어야 한다.

### Key Entities *(include if feature involves data)*

- **Speckit Feature**: 현재 worktree의 `specs` 영역 아래에 있는 하나의 기능 단위이다. 기능 이름, 상대 위치, 포함된 문서 목록, tasks 진행 요약을 가진다.
- **Speckit Document**: 기능에 속한 개별 산출물이다. 문서 유형, 표시 이름, 상대 위치, 읽기 가능 상태, 선택 상태를 가진다.
- **Task Progress Summary**: `tasks.md`에서 사용자가 볼 수 있는 작업 진행 요약이다. 완료 수, 전체 수, 미완료 수, 작업 없음 상태를 가진다.
- **Speckit Empty State**: 현재 worktree에 표시 가능한 Speckit 기능 또는 문서가 없을 때 사용자에게 보여주는 상태이다.
- **Document Read Error**: 특정 문서를 열거나 요약할 수 없을 때 해당 항목에 연결되는 사용자-visible 오류이다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`에 한정한다. Speckit 패널이 다른 앱에서도 필요하다는 근거가 생기기 전까지 cross-app 공유는 도입하지 않는다.
- **Frontend layering**: worktree session 화면 조합은 `pages`, Speckit 패널 선택과 문서 열기 상호작용은 `features`, Speckit 기능/문서/진행 요약 모델은 `entities`, 재사용 가능한 작은 UI 요소는 `shared` 또는 기존 `components/ui` 경계를 따른다.
- **Backend boundary**: worktree 파일 탐색과 문서 읽기 책임이 필요할 경우 Tauri 명령은 inbound adapter에 머물고, 현재 worktree 기준의 Speckit 문서 조회 규칙은 application service와 port 뒤에 둔다. 실제 파일 접근과 경로 검증은 infrastructure adapter가 책임진다.
- **Shared core vs UI**: 먼저 Speckit 기능 목록 구성, 문서 유형 분류, tasks 진행 요약 계산을 순수 모델로 검증한다. 공유 UI는 여러 화면에서 동일한 Speckit 탐색 경험이 필요해질 때만 검토한다.
- **Persistence and safety**: 파일 접근은 현재 worktree root 범위 안으로 제한되어야 하며, 상대 경로 정규화, root 이탈 방지, 읽기 실패 처리, 텍스트 크기 제한, UTF-8 처리 방침을 따라야 한다.
- **Documentation and Storybook**: 사용자 흐름이나 갱신 정책을 문서화할 경우 `docs/*.md`에 English filename과 Korean content로 작성한다. Speckit 패널 UI와 재사용 컴포넌트는 Storybook에 organisms 또는 pages 범주로 기본, 빈 상태, 오류, 긴 목록, tasks 진행 요약 상태를 등록한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Speckit 구조가 있는 worktree에서 사용자는 worktree session 진입 후 5초 이내에 Speckit 기능 목록과 주요 문서 위치를 확인할 수 있다.
- **SC-002**: 사용자는 특정 기능의 `spec.md`, `plan.md`, `tasks.md` 중 원하는 문서를 2번 이하의 선택 동작으로 열 수 있다.
- **SC-003**: `tasks.md` 체크박스가 포함된 대표 기능 20개에 대해 진행 요약의 완료/전체 작업 수가 실제 문서와 100% 일치해야 한다.
- **SC-004**: `specs`가 없거나 비어 있는 worktree 10개를 열어도 Speckit 패널로 인해 worktree session 오류가 발생하는 사례가 0건이어야 한다.
- **SC-005**: Speckit 문서 추가, 수정, 삭제 후 사용자는 3초 이내 또는 명시적 갱신 1회 이내에 최신 목록 또는 상태 요약을 확인할 수 있다.
- **SC-006**: 사용성 검증에서 사용자의 90% 이상이 파일 트리 탐색 없이 현재 기능의 핵심 Speckit 문서를 찾는 데 성공해야 한다.

## Assumptions

- 대상 사용자는 Agentic Workbench에서 Speckit 기반 기능 작업을 수행하는 사용자이다.
- v1 범위는 현재 선택된 worktree의 Speckit 산출물을 탐색하고 기존 markdown 검토 흐름으로 여는 것이다.
- Speckit 기능 폴더는 기본적으로 `specs/*` 아래에 있으며, 기능별 주요 문서는 issue #111에 명시된 파일명과 하위 폴더를 기준으로 한다.
- 문서 편집 기능, spec 생성/plan/tasks 실행 기능, GitHub project 상태 변경 기능은 이번 feature의 범위가 아니다.
- tasks 진행 요약은 markdown 체크박스 작업 항목을 기준으로 하며, 일반 텍스트 목록이나 임의 진행률 표기는 진행률 계산 대상에 포함하지 않는다.
- 파일 변경 감지와 갱신은 기존 worktree session의 파일, Git, markdown 패널에서 사용자가 기대하는 방식과 일관되게 맞춘다.

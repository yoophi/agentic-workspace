# Tasks: Markdown 렌더링 품질 개선

**Input**: Design documents from `/specs/033-markdown-rendering-quality/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md),
[research.md](./research.md), [data-model.md](./data-model.md),
[rendering UI contract](./contracts/rendering-quality-contract.md),
[quickstart.md](./quickstart.md)

**Tests**: CommonMark/GFM 구조, 순수 parser·fixture 비교, 공유 viewer 및 주석 앵커는
컨스티튜션에 따라 테스트 우선으로 구현한다. MA와 AW는 공유 package 소비자 회귀를
검증한다.

**Organization**: 태스크는 사용자 스토리별로 묶어 각 기능을 독립적으로 구현·검증할
수 있게 한다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 서로 다른 파일을 수정하며 선행 태스크에 의존하지 않아 병렬 실행 가능
- **[Story]**: 해당 사용자 스토리(US1~US4)
- 모든 태스크는 정확한 파일 경로를 포함한다.

## Phase 1: Setup (공유 기반 준비)

**Purpose**: AST 기반 Markdown 해석과 fixture 검증에 필요한 직접 의존성과 public
export 경계를 준비한다.

- [X] T001 `packages/markdown-annotation-core/package.json`에 CommonMark AST 파싱과 AST 텍스트 추출에 필요한 직접 의존성을 추가한다.
- [X] T002 `pnpm-lock.yaml`에서 T001에 따른 workspace 의존성 잠금 파일을 갱신한다.
- [X] T003 `packages/markdown-annotation-core/src/index.ts`와 `packages/markdown-annotation-core/src/types/index.ts`에 새 렌더 구조·품질 fixture public export 경계를 선언한다.

---

## Phase 2: Foundational (모든 스토리의 차단 선행 조건)

**Purpose**: 전체 문서 AST를 기준으로 렌더·주석·품질 fixture가 동일한 구조와 식별자를
참조하도록 기반 타입과 변환 경계를 확립한다.

**⚠️ CRITICAL**: 이 단계가 끝나기 전에는 어떤 사용자 스토리도 구현하지 않는다.

- [X] T004 `packages/markdown-annotation-core/src/types/markdown-block.ts`에 source range, parent 관계 및 block metadata를 포함하는 호환 가능한 렌더 문서 블록 타입을 정의한다.
- [X] T005 [P] `packages/markdown-annotation-core/src/quality/types.ts`에 fixture id, 분류, 기대 구조, 기대 텍스트 및 annotation expectation 타입을 정의한다.
- [X] T006 `packages/markdown-annotation-core/src/parse/parse-markdown-to-blocks.test.ts`에 AST 기반 block id·source range·부모 관계의 결정성과 2,000개 구조 성능 기준을 먼저 검증하는 실패 테스트를 추가한다.
- [X] T007 `packages/markdown-annotation-core/src/parse/parse-markdown-to-blocks.ts`에 문서 전체 AST에서 주석 가능한 의미 블록을 추출하고 기존 소비자용 metadata를 유지하는 구현을 작성한다.

**Checkpoint**: 공유 문서 구조와 public 계약이 준비되어 US1~US4 구현을 시작할 수 있다.

---

## Phase 3: User Story 1 - 표준 Markdown 문서 정확히 미리보기 (Priority: P1) 🎯 MVP

**Goal**: 목록 lazy continuation, 중첩 목록, 여러 문단 항목과 지원 GFM 요소를 문서
전체 의미와 유효한 HTML 계층으로 표시한다.

**Independent Test**: 복합 목록 Markdown을 한 번 렌더링해 `ul/ol > li`, `ol[start]`,
`li > p + p`, 중첩 목록, 코드/표/GFM 요소 관계와 표시 텍스트를 확인한다.

### Tests for User Story 1 ⚠️

- [X] T008 [P] [US1] `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`에 lazy continuation, loose list, 중첩 순서·비순서 목록, 시작 번호, 코드·표·GFM 혼합의 유효 HTML 구조 테스트를 먼저 추가한다.
- [X] T009 [P] [US1] `packages/markdown-annotation-core/src/toc/extract-toc-entries.test.ts`에 AST 기반 heading·task metadata가 기존 TOC 결과를 유지하는 회귀 테스트를 추가한다.

### Implementation for User Story 1

- [X] T010 [US1] `packages/markdown-annotation-react/src/MarkdownViewer.tsx`를 원문 전체를 한 번 렌더링하고 실제 의미론적 요소에 block metadata를 연결하는 renderer로 전환한다.
- [X] T011 [US1] `packages/markdown-annotation-react/src/markdown-components.tsx`를 추가하여 list, list item, paragraph, heading, blockquote, code, table의 유효한 DOM·toolbar 배치를 제공한다.
- [X] T012 [US1] `packages/markdown-annotation-react/src/styles.css`에서 새 의미론적 목록·task list·toolbar layout이 본문 흐름과 중첩 목록을 깨지 않게 조정한다.
- [X] T013 [US1] `packages/markdown-annotation-core/src/toc/extract-toc-entries.ts`와 `packages/markdown-annotation-react/src/MarkdownViewer.tsx`에서 새 block metadata를 사용해 TOC 및 H1 task summary 호환성을 완성한다.
- [X] T014 [US1] `packages/markdown-annotation-react/src/MermaidExpandedView.tsx`와 `packages/markdown-annotation-react/src/MarkdownViewer.tsx`에서 fenced Mermaid 감지와 일반 code fallback이 새 renderer에서도 동일하게 동작하도록 연결한다.

**Checkpoint**: CommonMark/GFM 복합 문서가 올바른 계층으로 렌더링되며 TOC·task summary·Mermaid가 유지된다.

---

## Phase 4: User Story 2 - 주석을 보존한 정확한 문서 읽기 (Priority: P1)

**Goal**: 새 의미론적 문서 구조에서 block/부분 선택 주석이 정확한 대상에 표시되고
편집·취소·문서 갱신 안전성이 유지된다.

**Independent Test**: 복합 목록의 부모 항목, 중첩 항목, 두 번째 문단에 각각 주석을
적용한 뒤 편집·취소하여 다른 블록과 목록 계층이 변하지 않음을 확인한다.

### Tests for User Story 2 ⚠️

- [X] T015 [P] [US2] `packages/markdown-annotation-react/src/build-viewer-annotation-maps.test.ts`를 추가해 전체 block, 부분 선택, stale anchor가 새 block id·rendered text 기준으로 올바르게 매핑되는 실패 테스트를 작성한다.
- [X] T016 [P] [US2] `packages/markdown-annotation-react/src/use-selection-anchors.test.ts`를 추가해 목록 안 여러 문단·중첩 요소의 선택이 올바른 의미 블록과 offset에 연결되는 실패 테스트를 작성한다.
- [X] T017 [P] [US2] `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`에 주석 mark·block toolbar가 `ul/ol/table`의 유효한 직접 자식 관계를 깨지 않는 렌더 회귀 테스트를 추가한다.

### Implementation for User Story 2

- [X] T018 [US2] `packages/markdown-annotation-react/src/build-viewer-annotation-maps.ts`에서 source/text 검증에 실패한 앵커를 다른 블록으로 재지정하지 않고 stale로 처리하도록 갱신한다.
- [X] T019 [US2] `packages/markdown-annotation-react/src/use-selection-anchors.ts`에서 의미론적 block 요소의 렌더 텍스트 offset과 source range를 이용해 선택 anchor를 생성하도록 갱신한다.
- [X] T020 [US2] `packages/markdown-annotation-react/src/MarkdownViewer.tsx`와 `packages/markdown-annotation-react/src/segment-text.ts`에서 inline annotation과 block action이 새 AST renderer의 텍스트 노드에만 적용되도록 구현한다.
- [X] T021 [US2] `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`와 `apps/markdown-annotator/src/pages/annotator/annotator-auto-reload.test.tsx`에서 새 block model로 주석 편집·취소·reload 호환성을 검증하고 소비 코드를 맞춘다.
- [X] T022 [US2] `apps/agentic-workbench/src/features/worktree-workspace/model/use-markdown-annotation-workspace.ts`와 `apps/agentic-workbench/src/features/worktree-workspace/model/use-markdown-annotation-workspace.test.ts`에서 새 anchor/stale 정책을 소비하도록 갱신한다.

**Checkpoint**: 주석은 복합 Markdown 구조를 손상시키지 않으며, 불일치한 앵커는 임의 이동 없이 안전하게 처리된다.

---

## Phase 5: User Story 3 - 렌더링 품질을 지속적으로 검증 (Priority: P2)

**Goal**: 표준에 근거한 독자 fixture 코퍼스로 구조·안전·주석 회귀를 fixture id별로
진단한다.

**Independent Test**: 품질 코퍼스 전체 실행에서 최소 20개 구조 사례, 10개 주석 결합
사례, 10개 불완전 입력 사례의 기대 구조를 비교하고 실패 fixture id와 기대 관계를
확인한다.

### Tests for User Story 3 ⚠️

- [X] T023 [P] [US3] `packages/markdown-annotation-core/src/quality/fixtures.test.ts`를 추가해 fixture id 중복, 분류별 최소 수량, 기대 구조 정의 완전성을 먼저 검증한다.
- [X] T024 [P] [US3] `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`에 품질 fixture별 HTML 관계·표시 텍스트·안전 fallback을 보고하는 파라미터화된 회귀 테스트를 추가한다.

### Implementation for User Story 3

- [X] T025 [US3] `packages/markdown-annotation-core/src/quality/fixtures.ts`에 목록 이어쓰기·빈 줄·중첩·시작 번호를 포함한 최소 20개 구조 사례, 최소 10개 주석 사례, 최소 10개 복원력·안전 사례를 추가한다.
- [X] T026 [US3] `packages/markdown-annotation-core/src/quality/assert-rendering-fixture.ts`와 `packages/markdown-annotation-core/src/quality/assert-rendering-fixture.test.ts`를 추가해 기대 의미 구조의 비교와 fixture id 중심 오류 보고를 구현한다.
- [X] T027 [US3] `packages/markdown-annotation-core/src/index.ts`와 `packages/markdown-annotation-react/src/test-fixtures.ts`에서 품질 fixture와 viewer 테스트 helper를 public/테스트 경계에 맞게 연결한다.

**Checkpoint**: 새 렌더링 결함은 최소 재현 fixture 하나로 추가하고, 한 번의 테스트 실행으로 구조 불일치를 식별할 수 있다.

---

## Phase 6: User Story 4 - AW 미리보기 공간에 맞춰 주석 영역 전환 (Priority: P2)

**Goal**: AW workspace에서 사용자가 주석 보조 열을 접고 펼쳐 본문 읽기 공간을
확보하되, 문서·주석·선택·작성 상태는 유지한다.

**Independent Test**: 주석이 있는 문서를 열고 보조 열을 숨겼다가 다시 표시해 본문
폭, 접근 가능한 전환 제어, annotation 수, selection/draft, agent prompt가 유지됨을
확인한다.

### Tests for User Story 4 ⚠️

- [X] T028 [US4] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`에 주석 영역의 기본 표시, 숨김 시 보조 열 제거·본문 확장, 재표시 테스트를 먼저 추가한다.
- [X] T029 [US4] `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-annotation-workspace.test.tsx`에 표시 상태 전환 중 annotation 수, selection anchor, draft/편집 상태 및 agent prompt 보존 테스트를 먼저 추가한다.

### Implementation for User Story 4

- [X] T030 [US4] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`에 기본값 `visible`의 AW-local 주석 영역 표시 상태와 접근 가능한 보이기/숨기기 제어를 구현한다.
- [X] T031 [US4] `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-annotation-workspace.tsx`에 보조 열 조건부 배치와 숨김 시 본문 열 확장을 구현하고 기존 annotation interaction state를 보존한다.

**Checkpoint**: AW에서 주석 보조 영역을 한 번의 조작으로 전환할 수 있고, 읽기 폭과 annotation workflow가 모두 유지된다.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: 문서화, Storybook, 소비자 회귀 검증, 성능·안전 검증을 완료한다.

- [X] T032 [P] `apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx`에 복합 목록, 주석 결합, 불완전 Markdown 복원 상태 Storybook 사례를 추가한다.
- [X] T033 [P] `docs/markdown-rendering-quality.md`에 품질 코퍼스, renderer/annotation 흐름, AW 주석 영역 전환과 검증 절차를 한국어 Mermaid 다이어그램으로 문서화한다.
- [X] T034 `packages/markdown-annotation-core/src/parse/parse-markdown-to-blocks.test.ts`와 `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`에서 2,000개 독립 구조 미리보기의 기존 성능 기준을 회귀 검증한다.
- [X] T035 `packages/markdown-annotation-core/package.json`, `packages/markdown-annotation-react/package.json`, `apps/markdown-annotator/package.json`, `apps/agentic-workbench/package.json`에 정의된 관련 check-types·test와 `apps/markdown-annotator` Storybook build를 실행해 [quickstart.md](./quickstart.md) 검증 결과를 기록한다.
- [X] T036 `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`와 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 최종 소비 경로를 점검해 app 간 직접 import, raw HTML 활성화, 위험 URL 활성화 회귀가 없음을 확인한다.

---

## Dependencies & Execution Order

### Phase Dependencies

```text
Setup (T001–T003)
  └─> Foundational (T004–T007)
       ├─> US1 정확한 구조 렌더링 (T008–T014)
       │    └─> US2 주석 보존 (T015–T022)
       ├─> US3 품질 코퍼스 (T023–T027)
       └─> US4 AW 주석 영역 전환 (T028–T031)
            └─> Polish (T032–T036; 모든 선택한 스토리 완료 후)
```

### User Story Dependencies

- **US1 (P1)**: Foundational 완료 후 시작하며 renderer와 metadata 기반을 제공한다.
- **US2 (P1)**: US1의 의미론적 renderer가 필요하다.
- **US3 (P2)**: Foundational 이후 시작할 수 있으나, 최종 viewer 구조 assertion은 US1·US2
  완료 후 실행한다.
- **US4 (P2)**: Foundational 이후 시작할 수 있으며, AW-local layout만 변경한다. 최종
  annotation 보존 검증은 US2와 함께 실행한다.

### Parallel Opportunities

- T005는 T004와 병렬 가능하며, T008과 T009는 US1 구현 전에 병렬로 작성할 수 있다.
- T015~T017은 US2 구현 전에 서로 다른 테스트 파일에서 병렬로 작성할 수 있다.
- T023과 T024는 US3 fixture 구현 전에 병렬로 작성할 수 있다.
- T028과 T029는 US4 구현 전에 병렬로 작성할 수 있다.
- T032와 T033은 모든 기능 동작이 안정된 뒤 서로 다른 산출물에서 병렬 수행할 수 있다.

## Implementation Strategy

### MVP First (US1만)

1. T001~T007로 AST 기반 공유 block model을 확립한다.
2. T008~T014로 복합 Markdown을 유효한 HTML 계층으로 렌더링한다.
3. US1의 구조·TOC·Mermaid 회귀 테스트를 통과해 독립적으로 시연한다.

### Incremental Delivery

1. US2를 추가해 주석 selection·편집·취소와 stale 안전성을 복구한다.
2. US3 fixture 코퍼스로 이후 변경의 구조 회귀를 즉시 검출한다.
3. US4로 AW 패널의 읽기 공간과 annotation workflow를 개선한다.
4. Polish 단계에서 Storybook, 문서, 성능·안전, MA/AW 소비자 검증을 완료한다.

## Format Validation

모든 구현 태스크는 `- [ ] T### [P] [US#] 설명과 정확한 파일 경로` 형식을 사용한다.
Setup·Foundational·Polish 태스크는 사용자 스토리 label을 사용하지 않고, 사용자 스토리
태스크는 모두 해당 `[US#]` label을 포함한다.

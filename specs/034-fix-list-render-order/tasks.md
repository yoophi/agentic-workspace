---
description: "Task list for Markdown 목록 렌더링 순서 보존 (TDD)"
---

# Tasks: Markdown 목록 렌더링 순서 보존

**Input**: Design documents from `/specs/034-fix-list-render-order/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: 사용자가 **TDD**를 요청했고 공유 패키지의 순수 렌더 로직이므로 테스트는 필수다.
각 계약 테스트(CT)는 **먼저 작성해 FAIL을 확인**한 뒤 구현으로 통과시킨다.

**Organization**: 태스크는 user story별로 묶어 독립 구현·검증이 가능하도록 한다. 단, 본
수정은 공유 `MarkdownViewer` 한 곳을 바꾸므로 **US2 구현은 US1 구현(런 기반 렌더)을
공유**한다(테스트는 각기 독립 실행 가능).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 서로 다른 파일·의존 없음 → 병렬 가능
- **[Story]**: US1 / US2 (setup·foundational·polish는 라벨 없음)
- 모든 태스크에 정확한 파일 경로 포함

## Path Conventions

- 공유 렌더러: `packages/markdown-annotation-react/src`
- 공유 core(무변경, 검증만): `packages/markdown-annotation-core/src`
- 소비 앱: `apps/markdown-annotator`(MA), `apps/agentic-workbench`(AW)
- 문서: `docs/markdown-rendering-quality.md`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: TDD 기준선과 공유 테스트 하네스 준비

- [X] T001 [P] 현재 그린 기준선 확보: `pnpm --filter @yoophi/markdown-annotation-react test`
  를 실행해 기존 통과 수(react 86)를 기록한다(변경 후 회귀 비교 기준).
- [X] T002 [P] 공유 테스트 하네스 생성 `packages/markdown-annotation-react/src/list-render-order.test.tsx`:
  `MarkdownViewer`·`parseMarkdownToBlocks`·최소 `components`(Button/Tooltip 스텁)와,
  `renderToStaticMarkup`로 HTML을 만들어 순서/구조를 단언하는 헬퍼를 둔다(아직 단언 없음).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 두 스토리 구현이 공유하는 동작 보존 리팩터

**⚠️ CRITICAL**: 이 단계 완료 전에는 스토리 구현을 시작하지 않는다.

- [X] T003 `packages/markdown-annotation-react/src/MarkdownViewer.tsx`에서 `renderList`의
  본문 중 "항목 배열 → ordered 경계 그룹화 → `MarkdownList` 렌더" 부분을
  `renderItems(items: MarkdownBlock[])` 헬퍼로 추출하고, `renderList(parentId)`는
  `renderItems(blocksByParent.get(parentId) ?? [])`를 호출하도록 바꾼다. **동작 불변**
  (기존 react 테스트 전부 그린 유지)이어야 한다.

**Checkpoint**: 리팩터 후 기존 테스트가 모두 통과하면 스토리 구현 시작 가능.

---

## Phase 3: User Story 1 - 제목·문단으로 구분된 목록의 순서 보존 (Priority: P1) 🎯 MVP

**Goal**: 중간 콘텐츠로 분리된 최상위 목록이 병합되지 않고 원문 순서대로 렌더된다.

**Independent Test**: 문단/제목으로 분리된 목록 문서를 렌더해 콘텐츠 순서가 원문 블록
순서와 일치하고, 분리된 목록이 별도 목록으로 표시되는지 확인.

### Tests for User Story 1 (TDD — 먼저 작성하고 FAIL 확인) ⚠️

- [X] T004 [US1] `list-render-order.test.tsx`에 US1 순서 테스트를 작성하고 FAIL 확인:
  CT-1(`- Apple`/문단/`- Banana` → Apple<Middle<Banana, 목록 2개),
  CT-2(`## Setup`+목록 / `## Usage`+목록 → 각 목록이 제 제목 뒤, Usage 제목이 그 목록
  항목보다 앞), CT-9(목록이 서로 다른 H1 챕터에 분산 → 각 목록이 제 챕터 뒤, TaskSummary
  위치 유지).
- [X] T005 [US1] `list-render-order.test.tsx`에 핵심 변경 안전 테스트를 작성하고 확인:
  CT-7(목록 항목 렌더 시 React key 경고 없음), CT-8(분리된 두 목록 중 두 번째 목록
  항목에 단 주석이 순서 보존 후에도 해당 항목에 유지).

### Implementation for User Story 1

- [X] T006 [US1] `packages/markdown-annotation-react/src/MarkdownViewer.tsx`에 런 기반
  최상위 렌더 구현: `blocks`를 1회 순회해 **최상위 `list-item` 런**을 계산한다(런 경계는
  **비-`list-item` 타입 블록만**; 중첩 `list-item`은 경계 아님). 각 런은 **첫 항목의
  인덱스에서 `renderItems(run)`으로 렌더**하고 그 외 `list-item`은 `null`을 반환한다.
  기존 `blocks.findIndex(...) === blocks.findIndex(...)` 조건을 제거한다. 렌더되는 각 항목에
  `key={item.id}`를 부여한다. → T004·T005가 통과해야 한다.
- [X] T007 [US1] `pnpm --filter @yoophi/markdown-annotation-react test`로 US1 테스트 통과와
  기존 테스트 무회귀(T001 기준선 대비)를 확인한다.

**Checkpoint**: 순서 버그 해소 = MVP. 독립 검증 가능.

---

## Phase 4: User Story 2 - 순서/비순서 경계와 중첩 보존 (Priority: P2)

**Goal**: 목록 종류·시작 번호·중첩 관계가 원문 의미대로 유지되고, 중첩 항목이 두 최상위
항목 사이에 있어도 두 항목이 하나의 목록으로 유지된다.

**Independent Test**: 순서/비순서 전환, 시작 번호, 중첩+형제, 중첩-사이-최상위 문서를
렌더해 종류·번호·중첩·비분리를 확인.

> **의존성 주의**: US2 구현은 US1의 런 기반 렌더(T006)를 공유한다. 테스트는 독립 실행
> 가능하나 코드는 US1 완료 후 검증/보강한다.

### Tests for User Story 2 (TDD) ⚠️

- [X] T008 [US2] `list-render-order.test.tsx`에 US2 테스트를 작성하고 FAIL/동작 확인:
  CT-3(비순서 다음 `3.` 시작 순서 목록 → `ul`과 `ol[start=3]` 분리),
  CT-4(부모/자식 중첩 + 부모의 뒤 형제 → 중첩 유지 및 형제 순서 유지),
  CT-6(`- A\n  - A1\n- B` → A·B가 하나의 `ul`, A 안에 A1 중첩; 두 목록으로 오분리 안 됨).

### Implementation for User Story 2

- [X] T009 [US2] `packages/markdown-annotation-react/src/MarkdownViewer.tsx`의 `renderItems`
  런 내부 그룹화(ordered 경계 분리, `orderedStart` 적용)와 중첩 재귀가 CT-3/CT-4/CT-6를
  만족하는지 확인하고, 미흡한 경우에만 그룹화/중첩 로직을 보강한다(대개 T006로 충족).
- [X] T010 [US2] `pnpm --filter @yoophi/markdown-annotation-react test` 및 기존
  `MarkdownViewer.test.tsx`의 시맨틱 목록 테스트(`<ul><li>`/`<ol start>`)가 그린인지 확인.

**Checkpoint**: US1·US2 모두 독립적으로 통과.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: 성능·문서·Storybook·교차 앱 검증

- [X] T011 [P] `list-render-order.test.tsx`에 CT-5 성능 테스트 추가: 2,000개 분리/일반
  목록 항목 렌더가 성능 예산(<2,000ms) 내이며 `item 2000`을 포함한다.
- [X] T012 [P] MA Storybook 사례 추가 `apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx`:
  제목·문단으로 분리된 목록이 원문 순서대로 렌더되는 "SeparatedListsOrder" 스토리.
- [X] T013 [P] `docs/markdown-rendering-quality.md`에 최상위 목록 순서 보존/런 경계 규칙
  섹션을 한국어로 추가.
- [X] T014 교차 앱 원자적 검증 실행(공유 UI 변경):
  `pnpm --filter @yoophi/markdown-annotation-core check-types && pnpm --filter @yoophi/markdown-annotation-core test`,
  `pnpm --filter @yoophi/markdown-annotation-react check-types`,
  `pnpm --filter @yoophi/markdown-annotator check-types && pnpm --filter @yoophi/markdown-annotator test && pnpm --filter @yoophi/markdown-annotator build-storybook`,
  `pnpm --filter @yoophi/agentic-workbench check-types && pnpm --filter @yoophi/agentic-workbench test`.
- [X] T015 `specs/034-fix-list-render-order/quickstart.md`의 수동 검증(MA Storybook에서
  제목/문단 분리 목록 순서, 목록 항목 주석 유지) 실행.
- [X] T016 `MarkdownViewer.tsx`의 잔여 `findIndex` 로직 제거 등 정리, 앱 간 직접 import가
  없는지 확인.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup(Phase 1)**: 의존 없음. 즉시 시작.
- **Foundational(Phase 2, T003)**: Setup 이후. 모든 스토리를 블록.
- **US1(Phase 3)**: Foundational 이후. MVP.
- **US2(Phase 4)**: Foundational 이후. **구현은 US1(T006) 공유** → 실무상 US1 다음에 검증.
- **Polish(Phase 5)**: 원하는 스토리 완료 후.

### Within Each User Story

- CT 테스트를 먼저 작성해 **FAIL 확인 후** 구현(TDD).
- 공유 core → 공유 UI 순서 유지(core는 무변경).
- 구현 후 회귀 확인(기존 테스트 그린).

### Parallel Opportunities

- T001, T002 병렬 가능([P]).
- Polish의 T011(테스트 파일)·T012(MA 스토리)·T013(문서)는 서로 다른 파일 → 병렬 가능([P]).
- US 단계의 테스트/구현은 동일 파일(`list-render-order.test.tsx`, `MarkdownViewer.tsx`)을
  다루므로 대체로 순차 진행.

---

## Parallel Example: Setup

```bash
# 동시에 실행 가능:
Task: "T001 기준선 테스트 실행 (react 86 그린 기록)"
Task: "T002 list-render-order.test.tsx 테스트 하네스 생성"
```

## Parallel Example: Polish

```bash
Task: "T011 CT-5 성능 테스트 추가 (list-render-order.test.tsx)"
Task: "T012 MA SeparatedListsOrder 스토리 추가 (MarkdownViewer.stories.tsx)"
Task: "T013 docs/markdown-rendering-quality.md 순서 규칙 추가"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 Setup → Phase 2 Foundational(동작 불변 리팩터) → Phase 3 US1.
2. **STOP & VALIDATE**: 문단/제목 분리 문서에서 순서 보존을 독립 검증.
3. 준비되면 MA·AW에서 확인/데모.

### Incremental Delivery

1. Setup + Foundational → 기반 완료.
2. US1(순서 보존) → 독립 검증 → MVP.
3. US2(종류/중첩 보존) → 독립 검증.
4. Polish(성능·문서·Storybook·교차 앱 검증) → 마무리.

---

## Notes

- 본 수정은 **공유 `MarkdownViewer` 한 파일**이 핵심이라 US2 구현이 US1 구현을 공유한다.
  테스트는 스토리별로 독립 실행 가능하다.
- MA와 AW는 동일 공유 렌더러를 사용 → 두 앱 모두 검증(T014).
- 각 CT는 FAIL 확인 후 구현으로 통과시킨다(TDD). 태스크/논리 그룹마다 커밋 권장.
- core 파서는 변경하지 않는다(순서 결함은 렌더러 계층). 항목 내 다중 하위 목록 병합은
  out-of-scope(spec 참조).

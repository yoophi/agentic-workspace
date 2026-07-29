# Implementation Plan: Markdown 목록 렌더링 순서 보존

**Branch**: `033-markdown-rendering-quality` | **Date**: 2026-07-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/034-fix-list-render-order/spec.md`

## Summary

`MarkdownViewer`는 최상위 목록 항목을 렌더할 때 문서 전체의 모든 최상위 `list-item`을
한데 모아 "첫 번째 최상위 목록 항목" 위치에서 한 번에 그린다. 그 결과 중간에 제목·문단
등 다른 블록으로 분리된 여러 목록이 하나로 병합되고, 그 사이 콘텐츠보다 앞에 렌더되어
문서 순서가 깨진다. 이 계획은 렌더러가 **문서 순서(blocks 배열 순서)를 기준으로 연속된
최상위 목록 항목 런(run)을 제자리에서** 렌더하도록 수정하여 순서를 보존한다. core 파서는
이미 올바르므로(순서 판단은 배열 위치만 사용) 변경하지 않으며, 수정은 공유 React 렌더러
`MarkdownViewer` 한 곳에 국한한다.

이 렌더러는 **agentic-workbench(AW)와 markdown-annotator(MA)가 모두 공유**하며(MA는
`AnnotatorPage.tsx`에서 동일 `MarkdownViewer`를 사용, 자체 렌더러 없음), 한 번의 수정으로
두 앱에 동일하게 적용된다. 특히 MA는 제목+목록 구조의 문서를 주석하는 것이 주 용도이므로
이 수정의 핵심 수혜 앱이다. 두 앱 모두를 명시적 검증 대상으로 삼는다(원자적 교차 검증).

### 개발 방식: TDD

마크다운 렌더는 입력(원문/블록)과 출력(HTML 구조·순서)이 명확하므로 **테스트 우선(TDD)**
로 진행한다. 계약(CT-1~CT-9)을 먼저 실패하는 테스트로 작성하고, 그 테스트를 통과시키는
최소 구현으로 렌더 루프를 수정한 뒤 리팩터링한다. 각 CT는 `react-dom/server` 정적 렌더
문자열에 대한 순서/구조 단언으로 표현한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19

**Primary Dependencies**: `@yoophi/markdown-annotation-core`(블록 모델), React
(`react-dom/server`로 테스트 렌더), Vitest 4

**Storage**: N/A (순수 렌더링 로직)

**Testing**: Vitest, **TDD(테스트 우선)** — `packages/markdown-annotation-react`(렌더
순서·회귀 계약 CT-1~CT-9), `packages/markdown-annotation-core`(기존 파서 테스트 무손상),
소비 앱 `markdown-annotator`(MA)·`agentic-workbench`(AW)의 check-types/test 및 MA
Storybook

**Target Platform**: Tauri desktop(agentic-workbench) 및 web(markdown-annotator)의
브라우저 렌더

**Project Type**: 모노레포 공유 React UI 패키지 + 소비 앱 2종

**Performance Goals**: 2,000 블록 문서 렌더가 기존 성능 예산(테스트 기준 <2,000ms) 내
유지. 순서 보존 로직은 blocks 1회 순회(O(n))로 처리.

**Constraints**: `ul`/`ol`의 직접 자식은 항상 `li`(033의 시맨틱 계약 유지). core 블록
모델·주석 앵커 계약 불변. 기존 core 87 / react 86 테스트 무손상.

**Scale/Scope**: 단일 함수 영역(`MarkdownViewer` 렌더 루프 + 목록 그룹화 헬퍼) 수정과
회귀 테스트·Storybook·문서 갱신.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 변경은 `packages/markdown-annotation-react/src/
  MarkdownViewer.tsx`(및 필요 시 `markdown-components.tsx`)에 국한. 앱 간 직접 import
  없음. 소비는 `apps/agentic-workbench`, `apps/markdown-annotator`.
- **Feature-Sliced Frontend Architecture**: PASS — 변경 대상이 공유 UI 패키지의 렌더러
  이며 앱 레이어(app/pages/features/entities) 변경 없음.
- **Hexagonal Tauri Backend Architecture**: N/A — Rust/Tauri 변경 없음.
- **Shared Core Before Shared UI**: PASS — core(파서/모델)는 그대로 소비하고, 이미 공유
  중인 UI 렌더러의 결함만 수정. 새 공유 표면 추가 없음.
- **Atomic Cross-App Verification**: PASS — `packages/markdown-annotation-react` 변경이
  므로 해당 패키지 test/check-types + **두 소비 앱 모두**(markdown-annotator=MA,
  agentic-workbench=AW)의 check-types/test와 MA build-storybook을 검증 목록에 포함
  (quickstart.md). MA는 공유 `MarkdownViewer`를 그대로 사용하므로 수정이 자동 적용된다.
- **Documentation and Storybook**: PASS — `docs/markdown-rendering-quality.md`에 순서
  보존 규칙 추가, MA(markdown-annotator) Storybook에 "분리된 목록/순서 보존" 사례 추가.
- **Testing and Safety**: PASS — 순수 렌더 로직이므로 **TDD로** CT-1~CT-9를 실패 테스트로
  먼저 작성한 뒤 통과시킨다. fixture 기반 단위 테스트로 순서·병합·중첩·주석 앵커를 고정.
  파일/세션/권한 변경 없음(안전성 항목 N/A).

**Result**: 위반 없음. Complexity Tracking 불필요.

## Project Structure

### Documentation (this feature)

```text
specs/034-fix-list-render-order/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── list-rendering-order-contract.md
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks output (not created here)
```

### Source Code (repository root)

```text
packages/markdown-annotation-react/src/
├── MarkdownViewer.tsx          # 렌더 루프: 최상위 목록 런(run) 제자리 렌더 + 마커(순서번호/‑) 렌더
├── markdown-components.tsx     # MarkdownList (ul/ol 경계) — 재사용, 변경 최소
├── styles.css                  # 목록 네이티브 마커 off(list-style:none) — 마커 중복 방지
└── list-render-order.test.tsx  # 순서 보존·병합 방지·마커(CT-1~9, MK-1~3) 회귀 테스트 (신규)

packages/markdown-annotation-core/src/
└── quality/fixtures.ts         # (선택) 분리된 목록 fixture 보강 — 순서 검증은 react에서

apps/markdown-annotator/src/stories/molecules/
└── MarkdownViewer.stories.tsx  # "분리된 목록" 사례 스토리 추가

docs/
└── markdown-rendering-quality.md  # 순서 보존 규칙 반영
```

**Structure Decision**: 결함은 공유 React 렌더러의 렌더 순서 로직에만 존재하므로 수정을
`packages/markdown-annotation-react`에 국한한다. core 블록 모델(`order`, `parentId`,
`sourceRange`)은 정확하므로 재사용하며, 두 소비 앱은 렌더러를 그대로 사용해 자동으로
수정 효과를 받는다.

## Implementation Approach (개요)

현재 렌더 루프는 `blocks.map`에서 최상위 `list-item` 중 문서상 첫 항목일 때만
`renderList()`(문서 전체 최상위 항목을 모아 렌더)를 호출한다. 이를 다음으로 대체한다.

1. **런(run) 감지 — 정확한 규칙**: `blocks`를 문서 순서로 1회 순회하며 "현재 런"을
   누적한다.
   - 최상위 `list-item`(`parentId === undefined`) → 현재 런에 추가(없으면 새 런 시작).
   - 중첩 `list-item`(`parentId !== undefined`) → **무시한다. 런을 시작하지도 끊지도
     않는다**(부모의 재귀 렌더로 그려짐).
   - 그 외 비-`list-item` 블록(heading/paragraph/code/table/blockquote/hr) → **현재
     런을 닫는다**(이 블록이 유일한 런 경계다).

   > ⚠️ 재검증에서 확인된 함정: `- A\n  - A1\n- B`처럼 중첩 항목(A1)이 두 최상위 항목
   > (A, B) 사이의 flat 배열에 끼는 경우, A와 B는 **같은 런**이다. 따라서 "직전 블록이
   > 최상위 목록 항목이 아니면 새 런"이라는 단순 규칙은 틀리다. 런 경계는 오직 비-
   > `list-item` 블록으로만 정의해야 한다. 또한 list item 내부의 blockquote/code/table은
   > 파서가 항목 `content`로 병합하므로 별도 블록으로 방출되지 않는다(경계 아님).

2. **런 렌더 위치**: 각 런은 그 첫 항목의 원문 인덱스에서 렌더한다. 런의 나머지 최상위
   항목과 모든 중첩 항목은 `blocks.map`에서 `null`을 반환한다. (구현: 첫 항목 인덱스 →
   런 매핑을 precompute.)
3. **런 내부 그룹화**: 런 안에서 순서/비순서(`ordered`)가 바뀌는 경계마다 별도
   `MarkdownList`(`ul`/`ol`)로 분리하고, 순서 목록은 `orderedStart`를 적용한다(기존
   그룹화 규칙 재사용). 이를 위해 현 `renderList(parentId)` 본문에서 항목 목록을 받는
   `renderItems(items)` 헬퍼를 분리하고, 중첩 재귀는 `renderItems(blocksByParent.get(id))`
   로 유지한다.
4. **중첩**: 각 항목의 중첩 자식은 기존 `renderList(item.id)` 재귀를 그대로 사용한다.
   같은 부모의 자식 항목은 파서상 항상 연속이며(문단 등은 항목 content로 병합됨) 사이에
   비-`list-item` 블록이 끼지 않으므로 중첩 수준에는 순서 결함이 없다.
5. **`key` 누락 수정**: 현재 `itemsInList.map((item) => renderBlock(item, ...))`는 각
   항목에 React `key`를 주지 않아 경고와 재조정 위험이 있다. 렌더 시 각 항목에
   `key={item.id}`를 부여한다.
6. **파생 표시 위치 유지**: TaskSummary는 H1 heading 인덱스(또는 preamble -1)에만 삽입
   되며(`countTasksByH1Chapter`), 그 키는 결코 `list-item` 인덱스와 겹치지 않는다. 런을
   제자리에 렌더하면 heading·비목록 블록의 인덱스 위치가 유지되어 TaskSummary 삽입도
   그대로 정확하다.

이로써 O(n) 단일 순회로 문서 순서가 보존되고, 비용이 큰 `findIndex` 이중 탐색도 제거된다.

### 마커 렌더링 (수렴 반영, FR-008·FR-009)

앱 실행 검증 중 발견된 마커 중복을 함께 고쳤다. viewer는 항목마다 커스텀 마커(비순서
`-`, 순서 `N.`, 작업 항목 체크박스)를 flex 레이아웃으로 직접 렌더하므로, `styles.css`에서
네이티브 목록 마커를 끈다(`list-style: none`). 끄지 않으면 `• -`, `1. 1.`처럼 중복된다.
순서 번호는 각 항목의 목록 내 위치(`orderedStart + index`)로 증가시키고, 마커 span은
`shrink-0 whitespace-nowrap`로 `N.`이 줄바꿈되지 않게 한다. 검증은 MK-1~3.

### 범위 밖 (알려진 한계)

한 목록 항목 안에서 문단으로 분리된 **여러 하위 목록**(예: `- P\n  - a\n\n  para\n\n  - b`)
은 파서(033)가 `para`를 부모 항목 content로 병합하고 `a`·`b`를 하나의 하위 목록으로
합친다. 이는 항목 content 병합 방식(033)의 특성이며 본 최상위 순서 수정의 범위가 아니다.

## Complexity Tracking

> 위반 없음 — 해당 없음.

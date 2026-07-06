# Research: AW workspace markdown preview TOC

**Feature**: 010-aw-preview-toc | **Date**: 2026-07-02

Technical Context에 NEEDS CLARIFICATION은 없다. spec Assumptions에서 설계로 미룬 배치 결정과 조립 방식 선택지를 확정한다.

## R1. TOC 배치 형태 (preview pane 내)

**Decision** *(구현 후 사용자 피드백으로 개정)*: annotation aside(본문 grid의 우측 열) **하단에 sticky로 배치**한다(`sticky bottom-4 mt-auto`, aside는 `flex flex-col`). 접힘 시 한 줄 토글 행("Contents" + chevron)이 preview 스크롤 위치와 무관하게 우측 하단에 상시 보이고, 펼치면 pinned 하단 기준으로 위쪽으로 목록(`max-height` + 자체 `overflow-y-auto`)이 열린다. 문서 어느 위치에서든 TOC에 접근할 수 있다.

**최초 결정(폐기)**: preview pane 최상단 in-flow 접이식 섹션. 구현·검증까지 완료했으나, 스크롤을 내리면 TOC가 화면에서 벗어나 재탐색 시 위로 돌아가야 하는 단점이 있어 사용자 요청으로 우측 하단 sticky로 변경했다. sticky 위치가 aside 열 내부(주로 빈 공간)라 본문 텍스트를 가리지 않는다 — 최초 결정에서 sticky를 기각한 사유(스크롤 타겟 가림)는 상단 sticky에 해당하며, 하단 aside sticky에는 적용되지 않는다(`scrollIntoView(block: "start")` 타겟은 상단 정렬).

**Rationale**:
- **overlay 기각**: FR-009·US2-3이 "본문과 TOC가 겹치거나 잘리지 않아야" 한다고 명시 — 오버레이는 본질적으로 본문을 가린다.
- **sticky 기각**: sticky 패널은 스크롤 시 콘텐츠 위에 떠서 `scrollIntoView(block: "start")`로 이동한 heading을 가린다. 공유 viewer에 scroll-margin을 추가하는 회피는 공유 패키지 무변경 원칙에 어긋난다.
- **본문 grid 3열화 기각**: preview는 xl에서 이미 `[minmax(0,1fr)_20rem]`(본문+annotation 열)이고 최소 폭 360px까지 좁아진다. 상시 열 추가는 본문 폭을 잠식해 US2와 상충.
- in-flow 접기 행은 높이 약 2rem만 차지해 "접힘 시 본문 표시 영역 실질 동일"(SC-004)을 만족하고, 펼침 상태에서도 겹침이 없다. 좁은 폭에서도 세로로만 확장되므로 안전하다(FR-009).
- preview header(파일명 표시)는 건드리지 않아 header 레이아웃 회귀 위험이 없다.

**Alternatives considered**: header 우측 토글 버튼 + popover(겹침 금지 위배), 좌측 상시 열(폭 잠식), MA처럼 패널 좌측 열(workspace는 ResizablePanel 구조라 열 추가 비용·밀도 부담 큼). 모두 기각.

## R2. 앱 로컬 컴포넌트 분리

**Decision**: 접이식 컨테이너를 `MarkdownPreviewToc`로 분리해 `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-preview-toc.tsx`에 둔다. 내부에서 공유 `MarkdownToc`를 사용하고, props는 `{ entries, onEntrySelect, defaultOpen?, className? }`. `entries`가 비면 `null`을 반환한다(토글 행 포함 미렌더, FR-005).

**Rationale**:
- `worktree-workspace-panel.tsx`는 이미 1,829줄 — 인라인 조립은 비대화를 심화하고 테스트 불가능하다.
- 별도 컴포넌트면 `renderToStaticMarkup` 마크업 테스트와 molecules 스토리 등록이 가능하다(constitution: 재사용 UI Storybook 등록, 상태별 스토리).
- 접이식 컨테이너는 AW preview 레이아웃 전용 관심사이므로 앱 로컬이 맞다. MA(정본)에도 같은 요구가 수렴하면 그때 공유 승격을 검토한다(constitution IV — 성급한 공유 금지).

**Alternatives considered**: panel 인라인 조립(테스트·스토리 불가), 공유 패키지에 접이식 변형 추가(요구 수렴 전 성급한 공유). 기각.

## R3. 펼침 상태 관리와 파일 전환 리셋

**Decision**: 펼침 상태는 `MarkdownPreviewToc` 내부 `useState(defaultOpen ?? false)`로 관리한다. 부모(panel)는 `key={selectedFilePath}`를 부여해 파일 전환 시 컴포넌트를 재마운트시켜 기본 접힘으로 리셋한다. `defaultOpen`은 테스트/스토리에서 펼침 상태를 정적으로 렌더하기 위한 prop이다.

**Rationale**: 상태를 컴포넌트에 캡슐화하면 panel에 상태·핸들러가 늘지 않는다. `key` 재마운트는 React 관용 패턴으로, "파일 전환 시 기본 접힘 시작"(spec Assumptions)을 부수 로직 없이 보장한다. 문서 내용만 갱신(자동 새로고침)될 때는 `selectedFilePath`가 그대로라 펼침 상태가 유지되고 entries만 갱신된다(FR-006과 부합).

**Alternatives considered**: panel에 `isTocOpen` useState + `useEffect` 리셋(상태 분산, effect 추가), 상태 저장(persist — spec이 명시적으로 비저장). 기각.

## R4. 토글/TOC 노출 조건

**Decision**: panel은 `previewQuery.data`가 있을 때만(`isLoading`/`isError`/미선택 분기 바깥) `MarkdownPreviewToc`를 렌더하고, 컴포넌트 자체도 `entries.length === 0`이면 `null`을 반환한다. `tocEntries`는 `useMemo(() => extractTocEntries(blocks), [blocks])`로 산출한다.

**Rationale**: 이중 방어로 FR-005(빈 문서·로딩/오류/미선택 상태에서 TOC UI 미렌더)를 만족한다. 기존 preview 분기(EmptyPanel/InlineState/데이터) 구조를 그대로 활용하므로 회귀 위험이 낮다. `blocks`는 이미 `previewQuery.data?.content` 기준 memo이므로 파일 전환·자동 새로고침 시 TOC가 자동 갱신된다(FR-006).

## R5. 스크롤 연결

**Decision**: `onEntrySelect` → `scrollToBlock(previewPaneRef.current, entry.blockId)` 연결. 공유 helper의 기존 동작(`data-block-id` 셀렉터, `block: "start"`, `prefers-reduced-motion` 존중)을 그대로 사용한다.

**Rationale**: preview 본문 블록은 공유 `MarkdownViewer`가 이미 `data-block-id`를 노출한다. `previewPaneRef`가 스크롤 컨테이너(`overflow-auto`)이므로 MA와 동일한 방식이 그대로 동작한다. TOC 섹션이 in-flow 최상단에 있어 스크롤 후 화면에서 벗어나지만, 이는 겹침 없는 배치의 자연스러운 결과이며 다시 위로 스크롤하면 재사용 가능하다.

## R6. 테스트·Storybook 전략

**Decision**:
- unit test(`markdown-preview-toc.test.tsx`, `renderToStaticMarkup`): 빈 entries → 빈 문자열, 기본(접힘) → 토글 행만 있고 목록 없음, `defaultOpen` → 목록·`data-toc-block-id` 렌더, entries 순서 보존.
- Storybook(`stories/molecules.stories.tsx`): 접힘(기본)/펼침(`defaultOpen`)/긴 목록(자체 스크롤)/빈 entries 4개 스토리 추가. entries는 `parseMarkdownToBlocks` + `extractTocEntries` 실출력으로 생성.
- 클릭 토글·스크롤의 통합 동작은 dev 앱 수동 검증(quickstart)으로 커버 — AW 테스트 환경에 jsdom을 새로 도입하지 않는다.

**Rationale**: specs/009와 동일한 원칙(기존 테스트 환경 유지, DOM 의존 최소화). 순수 추출 로직·공유 컴포넌트·스크롤 helper는 specs/009에서 이미 테스트되어 있어 AW에서는 조립 상태의 마크업 검증이면 충분하다.

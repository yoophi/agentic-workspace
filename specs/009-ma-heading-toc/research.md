# Research: MA markdown viewer heading 기반 TOC

**Feature**: 009-ma-heading-toc | **Date**: 2026-07-02

Technical Context에 NEEDS CLARIFICATION은 없으며, spec의 Assumptions에서 설계 단계로 미룬 결정과 구현 방식 선택지를 여기서 확정한다.

## R1. TOC 배치 형태 (사이드 패널 vs 접이식)

**Decision**: AnnotatorPage 문서 pane 좌측에 **접이식 사이드 패널**로 배치한다. 현재 `grid-cols-[minmax(0,1fr)_420px]` 레이아웃(문서 | annotation aside)에 TOC 열을 추가해 `[auto_minmax(0,1fr)_420px]` 형태로 확장하고, 토글 버튼으로 접고 펼 수 있게 한다. TOC 대상 entry가 0개면 열 자체를 렌더하지 않는다 (FR-006).

**Rationale**:
- 이슈가 "사이드 패널 또는 접이식 영역" 양쪽을 허용하므로 두 속성을 겸비한 형태(기본 펼침 + 접기 가능)가 가장 무난하다.
- 우측 aside(420px)는 annotation/prompt 작업 영역으로 이미 포화 상태라 TOC를 끼워 넣으면 핵심 워크플로를 방해한다. 문서 탐색 보조 기능은 문서와 같은 쪽(좌측)이 자연스럽다.
- TOC 패널은 자체 스크롤(`overflow-y-auto`)을 가져 본문 ScrollArea와 간섭하지 않는다 (spec Edge Case).

**Alternatives considered**:
- **우측 aside에 세 번째 탭(TOC) 추가**: annotation 작업 중 TOC를 보려면 탭 전환이 필요해 "빠른 이동" 목적과 상충. 기각.
- **문서 위 floating overlay**: 본문 텍스트 선택(annotation의 핵심 인터랙션)과 겹칠 위험. 기각.
- **문서 Card 내부 상단 목차 블록**: 긴 문서에서 스크롤하면 사라져 상시 접근 불가. 기각.

## R2. 스크롤 이동 메커니즘

**Decision**: TOC 항목 클릭 시 `container.querySelector('[data-block-id="<id>"]')`로 대상 요소를 찾아 `scrollIntoView({ behavior, block: "start" })`를 호출하는 `scrollToBlock` helper를 `markdown-annotation-react`에 둔다. `behavior`는 `prefers-reduced-motion: reduce`일 때 `"auto"`, 아니면 `"smooth"`. 컨테이너(문서 pane ref)는 앱이 주입한다.

**Rationale**:
- `MarkdownViewer`의 `BlockShell`이 이미 모든 블록에 `data-block-id`를 노출하므로(`MarkdownViewer.tsx:216`) 추가 DOM 변경이 필요 없다.
- `scrollIntoView`는 Radix ScrollArea viewport 내부에서도 가장 가까운 스크롤 조상 기준으로 동작하므로 offset 계산이 불필요하다.
- 블록 `id` 기반 셀렉터라 동일 텍스트 heading 중복 문제가 없다 (FR-005).
- helper를 앱이 아닌 react 패키지에 두는 이유: `data-block-id` DOM 계약의 소유자가 `MarkdownViewer`(패키지)이므로, 셀렉터 규약을 패키지 내부에 캡슐화해 앱이 DOM 세부에 결합하지 않게 한다.

**Alternatives considered**:
- **URL 해시 앵커(`#heading-slug`)**: URL 변경 없이 스크롤만 한다는 가정(spec Assumptions)에 어긋나고 slug 중복 처리 필요. 기각.
- **offset 수동 계산 + `scrollTo`**: ScrollArea 내부 좌표 계산이 필요해 취약. 기각.
- **react-scroll 등 라이브러리**: 신규 의존성 불필요. 기각.

## R3. heading 텍스트 inline 서식 제거 방식

**Decision**: `markdown-annotation-core`에 정규식 기반 `stripInlineMarkdown(text)` 순수 함수를 추가한다. 처리 대상: 이미지(`![alt](url)` → alt), 링크(`[text](url)` → text), bold/italic(`**`, `__`, `*`, `_`), inline code(`` ` ``), strikethrough(`~~`). 처리 후 trim.

**Rationale**:
- heading 블록의 `content`는 `#` 제거 후의 raw inline markdown이므로 TOC 표시용 plain text 변환이 필요하다 (FR-008).
- core는 의존성 없는 순수 TS 패키지이므로 remark 계열 파서 도입은 과하다. heading 한 줄에 등장하는 inline 서식은 제한적이라 정규식으로 충분하며, 실패해도 "서식 기호가 보이는" 수준의 열화라 위험이 낮다.
- 순수 함수로 두면 fixture 테스트가 쉽고, 추후 formatter 등 다른 곳에서 재사용 가능하다.

**Alternatives considered**:
- **ReactMarkdown 렌더 후 textContent 추출**: React/DOM 의존이 core에 유입되고 렌더 비용 발생. 기각.
- **remark/mdast 파싱**: core에 신규 의존성 추가, 번들 증가. heading 1줄 처리에 과함. 기각.
- **서식 기호 그대로 표시**: FR-008 위반. 기각.

## R4. 컴포넌트 API 분배 (core / react / app)

**Decision**:
- **core**: `extractTocEntries(blocks: MarkdownBlock[]): TocEntry[]` — level 1~3 heading 블록만 골라 문서 순서대로 `{ blockId, level, text, startLine }` 반환. `text`는 `stripInlineMarkdown` 적용 결과.
- **react**: `MarkdownToc({ entries, onEntrySelect, className? })` — entries를 level별 들여쓰기로 렌더하는 presentational 컴포넌트. entries가 비면 `null` 반환. 스크롤은 직접 수행하지 않고 `onEntrySelect(entry)` 콜백만 호출. 별도로 `scrollToBlock(container, blockId, options?)` helper export.
- **app**: `AnnotatorPage`에서 `useMemo(() => extractTocEntries(blocks), [blocks])`로 entry 산출, `onEntrySelect` → `scrollToBlock(documentPaneRef.current, entry.blockId)` 연결, 접이식 패널 상태(useState) 관리.

**Rationale**:
- constitution IV: 순수 core 우선, 공유 UI는 앱 shell 비의존. `MarkdownToc`가 스크롤을 직접 하지 않고 콜백만 노출하면 컨테이너 ref, 접기 상태 등 앱 관심사와 완전히 분리된다.
- `MarkdownViewer`의 기존 `components` 주입 패턴(Button/Tooltip)은 TOC에는 불필요 — TOC는 단순 button 목록이라 주입 컴포넌트 없이 Tailwind 클래스로 충분하다.

**Alternatives considered**:
- **MarkdownViewer에 TOC 내장(prop으로 on/off)**: viewer 책임 비대화, 레이아웃(사이드 배치)은 앱 관심사라 viewer 내부에 넣을 수 없음. 기각.
- **추출 로직을 react 패키지에 배치**: 순수 로직은 core가 원칙(constitution IV). 기각.

## R5. 테스트 전략

**Decision**:
- core: `extractTocEntries` / `stripInlineMarkdown`에 fixture 기반 vitest unit test (h1~h6 혼재, heading 없음, 동일 텍스트 중복, 서식 포함, frontmatter 문서 등 `parseMarkdownToBlocks` 실출력 사용).
- react: `MarkdownToc`는 기존 관례(`MarkdownViewer.test.tsx`)대로 `renderToStaticMarkup`으로 마크업 검증 (entries 렌더 순서, level별 들여쓰기 속성, 빈 entries → null). `scrollToBlock`은 fake ParentNode(`querySelector` + `scrollIntoView` 스텁) 주입으로 셀렉터와 옵션 호출 검증 — jsdom 도입 없이 node 환경 유지.
- app: `check-types` + 기존 `vitest run --passWithNoTests`. Storybook 스토리로 시각 상태(기본/긴 목록/h3 시작/빈 상태 미렌더) 문서화.

**Rationale**: 패키지에 jsdom/testing-library를 새로 도입하지 않고 기존 테스트 환경을 유지한다. DOM 의존을 `scrollToBlock` 한 함수로 얇게 격리했기 때문에 스텁 주입으로 충분히 검증 가능하다.

**Alternatives considered**: jsdom 환경 추가(클릭→스크롤 통합 테스트) — 의존성·설정 추가 대비 이득이 작고, 통합 동작은 Storybook/수동 검증(quickstart)으로 커버. 기각.

## R6. 들여쓰기 규칙 (h1 없이 시작하는 문서)

**Decision**: 들여쓰기는 heading level 절대값 기준 고정(`(level - 1) × 단위`)으로 하고, 문서 구조에 따른 정규화(예: h3만 있는 문서를 depth 0으로 승격)는 하지 않는다.

**Rationale**: spec US2 시나리오 2("각 level에 해당하는 들여쓰기")와 일치하고, 구현이 단순하며, 사용자가 원문 heading level을 그대로 유추할 수 있다.

**Alternatives considered**: 최소 level 기준 상대 정규화 — 시각적으로는 컴팩트하지만 원문 level 정보가 왜곡되고 스펙 시나리오와 불일치. 기각.

# Quickstart: MA heading TOC 검증 가이드

**Feature**: 009-ma-heading-toc | **Date**: 2026-07-02

구현 완료 후 이 문서의 절차로 기능을 end-to-end 검증한다. API 상세는 [contracts/toc-api.md](./contracts/toc-api.md), 데이터 규칙은 [data-model.md](./data-model.md) 참조.

## 사전 조건

```bash
pnpm install
```

## 1. 패키지 단위 검증 (Atomic Cross-App Verification)

```bash
# core: TOC 추출/서식 제거 unit test + 타입 체크
pnpm --filter @yoophi/markdown-annotation-core test
pnpm --filter @yoophi/markdown-annotation-core check-types

# react: MarkdownToc 마크업 + scrollToBlock helper test + 타입 체크
pnpm --filter @yoophi/markdown-annotation-react test
pnpm --filter @yoophi/markdown-annotation-react check-types

# 소비 앱: 타입 체크 + 테스트
pnpm --filter markdown-annotator check-types
pnpm --filter markdown-annotator test

# 소비 앱 2: agentic-workbench도 markdown-annotation 패키지를 소비하므로 함께 검증
pnpm --filter agentic-workbench check-types
pnpm --filter agentic-workbench test
```

**기대 결과**: 전부 통과. 특히 기존 `MarkdownViewer`·파서·anchor 관련 테스트가 변경 없이 통과해야 한다 (SC-005, FR-007).

## 2. Storybook 시각 검증

```bash
pnpm --filter markdown-annotator storybook   # port 6007
```

`molecules/MarkdownToc` 스토리에서 확인:

- 기본: h1/h2/h3 혼재 entry가 순서대로, level별 들여쓰기로 표시
- 긴 목록: entry 다수일 때 TOC 자체 스크롤 동작
- h3 시작 문서: level 절대값 기준 들여쓰기 유지
- 빈 entries: 아무것도 렌더되지 않음

## 3. 앱 수동 검증 시나리오

```bash
pnpm --filter markdown-annotator dev
```

| # | 시나리오 | 기대 결과 | 근거 |
|---|----------|-----------|------|
| S1 | h1~h3가 여러 개 있는 예제 문서 열기 | 문서 pane 좌측에 TOC 패널 표시, 문서 순서·들여쓰기 정확 | US1, US2 |
| S2 | 중간의 TOC 항목 클릭 | viewer가 해당 heading으로 스크롤 (부드러운 이동) | FR-005 |
| S3 | h4~h6 포함 문서 열기 | h4~h6는 TOC에 없음 | FR-004 |
| S4 | 동일 텍스트 heading이 2회 이상 등장하는 문서에서 두 번째 항목 클릭 | 두 번째 heading 위치로 정확히 이동 | US1-4 |
| S5 | heading 없는 문서(또는 h4~h6만) 열기 | TOC 패널/열 자체가 렌더되지 않음 | FR-006 |
| S6 | TOC 접기 토글 클릭 | 패널이 접히고 본문 영역이 확장, 다시 펼치기 가능 | Assumptions |
| S7 | TOC로 이동 후 텍스트 선택 → annotation 추가 → prompt 출력 확인 | annotation line/offset 정보가 TOC 도입 전과 동일하게 산출 | FR-007, SC-005 |
| S8 | inline 서식(`**bold**`, `` `code` ``, 링크) 포함 heading | TOC에 서식 기호 없는 plain text 표시 | FR-008 |
| S9 | heading 100개 이상 문서 열기 | TOC 표시 체감 지연 없음, TOC 독립 스크롤 | SC-001 |
| S10 | 문서 맨 끝 heading 클릭 | 스크롤 가능한 최대 위치까지 이동, 오류 없음 | Edge Case |

## 4. 회귀 확인

```bash
pnpm turbo run check-types test --filter=...@yoophi/markdown-annotation-core --filter=...@yoophi/markdown-annotation-react
```

(turbo 필터가 환경에 없으면 1번 명령 세트로 대체.)

**완료 기준**: 1·2·3의 모든 항목 통과 + S7에서 anchor 회귀 없음.

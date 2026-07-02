# Quickstart: AW markdown preview TOC 검증 가이드

**Feature**: 010-aw-preview-toc | **Date**: 2026-07-02

구현 완료 후 이 절차로 end-to-end 검증한다. 조립 계약은 [contracts/aw-preview-toc-ui.md](./contracts/aw-preview-toc-ui.md), 상태 규칙은 [data-model.md](./data-model.md) 참조.

## 사전 조건

```bash
pnpm install
```

## 1. 자동 검증

```bash
# 대상 앱: 타입 체크 + 테스트 (MarkdownPreviewToc 마크업 테스트 포함)
pnpm --filter agentic-workbench check-types
pnpm --filter agentic-workbench test

# 공유 패키지 무변경 확인 (출력 없어야 함)
git diff --stat main -- packages/
```

**기대 결과**: 전부 통과, `packages/` diff 없음. 기존 annotation·선택 하이라이트 관련 테스트가 변경 없이 통과 (SC-006).

## 2. Storybook 시각 검증

```bash
pnpm --filter agentic-workbench storybook
```

`Molecules` 카테고리에서 `MarkdownPreviewToc` 스토리 확인:

- 접힘(기본): 한 줄 토글 행만 표시, 목록 없음
- 펼침(`defaultOpen`): level별 들여쓰기 목록 표시
- 긴 목록: max-height 내 자체 스크롤
- 빈 entries: 아무것도 렌더되지 않음

## 3. 앱 수동 검증 시나리오

```bash
pnpm --filter agentic-workbench dev   # 또는 tauri dev (worktree 세션 필요)
```

worktree 세션 → workspace 패널 → Markdown 탭에서:

| # | 시나리오 | 기대 결과 | 근거 |
|---|----------|-----------|------|
| S1 | h1~h3가 여러 개인 markdown 파일 선택 | preview 상단에 접힌 TOC 토글 행 표시, 본문 폭·위치는 기존과 동일 | US2-1, SC-004 |
| S2 | 토글 클릭으로 펼침 | 문서 순서·level 들여쓰기로 목차 표시 | US1-1 |
| S3 | 중간 TOC 항목 클릭 | preview가 해당 heading으로 스크롤 (부드러운 이동) | US1-2 |
| S4 | 동일 텍스트 heading 중복 문서에서 두 번째 항목 클릭 | 두 번째 heading 위치로 정확히 이동 | US1-4 |
| S5 | h4~h6 포함 문서 | h4~h6는 목차에 없음 | FR-002 |
| S6 | heading 없는 파일(또는 h4~h6만) 선택 | TOC 토글 행 자체가 나타나지 않음 | FR-005 |
| S7 | TOC 펼친 채 다른 markdown 파일로 전환 | 새 문서 기준 목차로 갱신되고 접힘 상태로 리셋 | FR-006, A9 |
| S8 | 파일 미선택/로딩/읽기 오류 상태 | TOC UI 미표시 | FR-005 |
| S9 | preview 패널 폭을 최소(360px 부근)로 좁힌 뒤 펼침 | 본문·TOC 겹침/잘림 없음, 목록 길면 자체 스크롤 | FR-009, US2-3 |
| S10 | TOC 이동 후 본문 텍스트 선택 → annotation 생성 → prompt 확인 | line/offset 앵커 정보가 기존과 동일하게 산출 | FR-008, SC-006 |
| S11 | agent 실행 등으로 문서 내용이 외부 변경(자동 새로고침) | 목차가 최신 heading으로 갱신, `selectedFilePath` 불변이면 펼침 상태 유지 | FR-006 |
| S12 | inline 서식(`**bold**`, `` `code` ``, 링크) 포함 heading | 목차에 서식 기호 없는 텍스트 표시 | FR-007 |

## 4. 완료 기준

1·2의 자동/시각 검증 전부 통과 + 3의 S1~S12 확인, 특히 S10(앵커 회귀 없음)과 `packages/` 무변경.

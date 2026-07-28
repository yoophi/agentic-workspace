# Quickstart: 목록 렌더링 순서 보존 검증

이 문서는 순서 보존 수정이 엔드투엔드로 동작함을 확인하는 실행 가이드다. 구현 세부는
`tasks.md`와 구현 단계에서 다룬다.

## 사전 조건

- 저장소 루트에서 `pnpm install` 완료.
- 브랜치: `033-markdown-rendering-quality` (worktree 내).

## 자동화 검증

렌더러 패키지(핵심 수정 대상):

```bash
pnpm --filter @yoophi/markdown-annotation-react check-types
pnpm --filter @yoophi/markdown-annotation-react test
```

기대 결과:
- 신규 계약 테스트(CT-1~CT-7, [contracts](./contracts/list-rendering-order-contract.md))
  통과. 특히 CT-6(중첩 항목이 두 최상위 항목 사이에 있어도 하나의 목록 유지)와
  CT-7(목록 항목 React key 경고 없음)을 포함한다.
- 기존 react 테스트 전부 통과(회귀 없음).

core 무손상 확인(파서는 변경하지 않음):

```bash
pnpm --filter @yoophi/markdown-annotation-core check-types
pnpm --filter @yoophi/markdown-annotation-core test
```

소비 앱 원자적 검증(공유 UI 변경이므로 **두 앱 모두 필수**):

```bash
# markdown-annotator (MA) — 공유 MarkdownViewer의 주 수혜 앱
pnpm --filter @yoophi/markdown-annotator check-types
pnpm --filter @yoophi/markdown-annotator test
pnpm --filter @yoophi/markdown-annotator build-storybook

# agentic-workbench (AW)
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
```

## 수동 검증(선택)

markdown-annotator Storybook에서 "분리된 목록" 사례를 열어 다음을 확인한다.

1. 다음 문서가 원문 순서대로 표시되는지:
   ```markdown
   ## Setup

   - install
   - build

   ## Usage

   - run
   - deploy
   ```
   → `install`, `build`가 Setup 아래, `run`, `deploy`가 Usage 아래에 표시되고,
   Usage 제목이 `run`보다 앞에 온다.
2. `- Apple` / 문단 / `- Banana` 문서에서 Apple → 문단 → Banana 순서로 보이고,
   Apple/Banana가 서로 다른 목록으로 표시되는지.
3. 목록 항목에 블록/부분 선택 주석을 달고, 순서가 뒤바뀌지 않으며 주석이 올바른 항목에
   유지되는지.

## 완료 기준

- 위 자동화 명령이 모두 통과.
- [spec.md](./spec.md)의 SC-001~SC-004 충족(순서 100% 일치, 회귀 테스트 통과, 기존
  테스트 무손상, 목록 종류·중첩 유지).

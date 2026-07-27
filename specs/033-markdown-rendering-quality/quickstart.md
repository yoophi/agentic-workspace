# 033 검증 가이드

## 사전 조건

- 저장소 루트에서 `pnpm install`을 완료한다.
- 구현 시 품질 fixture는 `packages/markdown-annotation-core/src/quality/`에,
  React 구조 검증은 `packages/markdown-annotation-react/src/`에 둔다.

## 자동 검증

```sh
pnpm --filter @yoophi/markdown-annotation-core check-types
pnpm --filter @yoophi/markdown-annotation-core test
pnpm --filter @yoophi/markdown-annotation-react check-types
pnpm --filter @yoophi/markdown-annotation-react test
pnpm --filter @yoophi/markdown-annotator check-types
pnpm --filter @yoophi/markdown-annotator test
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/markdown-annotator build-storybook
```

기대 결과:

- 품질 코퍼스는 fixture id별로 기대 구조와 실제 구조의 차이를 표시하고 전부 통과한다.
- 목록 lazy continuation, blank-line 문단, 중첩 목록, 시작 번호가 HTML 계층과
  텍스트 모두에서 보존된다.
- GFM 표·task list·취소선·자동 링크, Mermaid·wikilink·TOC가 기존 기대를 유지한다.
- annotation fixture에서 block/부분 선택의 edit·cancel 후 다른 블록이 바뀌지 않는다.
- raw HTML, 위험 URL, 불완전 Markdown fixture가 렌더링 중단이나 활성 콘텐츠 실행을
  일으키지 않는다.
- AW workspace 테스트는 주석 영역 표시/숨김에서 본문 열 확장, 접근 가능한 제어 이름,
  문서·주석·작성 중 상태 보존을 확인한다.

## 수동 확인

```sh
pnpm run dev:annotator
pnpm run dev:workbench
```

1. MA에서 복합 목록 fixture를 열고 lazy continuation이 첫 목록 항목 안에 표시되는지
   확인한다.
2. 부모 목록, 중첩 목록, 두 번째 문단에 각각 block/부분 선택 주석을 추가한 뒤 edit와
   cancel을 수행한다.
3. AW의 SpecKit 또는 workspace preview에서 같은 fixture를 열어 HTML 구조와 주석
   동작이 MA와 같은지 확인한다.
4. AW workspace에서 주석이 있는 문서를 열고 `주석 영역 숨기기`를 실행해 본문이 넓어지고
   주석 보조 열이 사라지는지 확인한다. 이어서 `주석 영역 보이기`를 실행해 같은 주석과
   agent prompt가 복원되는지 확인한다.
5. 주석 작성 또는 편집 dialog를 연 상태로 주석 영역을 전환해 문서, 선택, 초안이
   유지되는지 확인한다.
6. Storybook의 MarkdownViewer molecule에서 복합 목록·주석 결합·복원력 사례를 확인한다.

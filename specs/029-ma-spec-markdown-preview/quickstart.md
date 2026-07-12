# Quickstart: MA Spec Markdown Preview 검증

## 사전 조건

- repository root에서 Node.js와 Corepack을 사용할 수 있어야 한다.
- `corepack pnpm install`이 완료되어 있어야 한다.
- 데스크톱 filesystem 이동을 검증하려면 Rust/Tauri 개발 환경이 필요하다.

데이터와 동작 계약은 [data-model.md](./data-model.md), [markdown-preview-contract.md](./contracts/markdown-preview-contract.md)를 참고한다.

## 자동 검증

repository root에서 실행한다.

```sh
pnpm --filter @yoophi/markdown-annotation-core check-types
pnpm --filter @yoophi/markdown-annotation-core test
pnpm --filter @yoophi/markdown-annotation-react check-types
pnpm --filter @yoophi/markdown-annotation-react test
pnpm --filter @yoophi/markdown-annotator check-types
pnpm --filter @yoophi/markdown-annotator test
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/markdown-annotator build
pnpm --filter @yoophi/markdown-annotator build-storybook
```

예상 결과는 모든 command가 exit code 0으로 끝나고 package/app test가 모두 통과하는 것이다. build chunk size warning은 기능 실패가 아니지만 신규 오류는 허용하지 않는다.

## MA browser surface 실행

```sh
pnpm run dev:annotator
```

표시된 localhost 주소를 열고 다음을 확인한다.

1. 예제 선택 목록에 `SpecKit · Feature Specification`, `Implementation Plan`, `Data Model`, `Tasks`, `Requirements Checklist`가 보인다.
2. 각 예제를 선택하면 file name, 본문과 H1~H3 TOC가 새 문서 기준으로 바뀐다.
3. tasks/checklist 예제에서 완료/미완료 icon과 count가 본문 및 H1 TOC에 일치한다.
4. plan/data model 예제의 table, code block, Mermaid가 문서 흐름 안에서 표시된다.
5. `[[speckit-...]]` link를 선택하면 연결된 예제로 이동하고 누락 target에서는 현재 문서를 유지한다.
6. 한 줄·여러 줄 HTML5 주석은 숨고 inline/fenced code 안의 주석 문자열은 보존된다.

## Storybook 검증

```sh
pnpm run storybook:annotator
```

- `Molecules/MarkdownViewer/TaskListStates`: open/completed, nested, long, link/inline-code task와 chapter summary를 확인한다.
- `Molecules/MarkdownToc/TaskProgress`: H1 task count와 task 없는 H1 표시를 확인한다.
- light/dark theme에서 icon, text, focus 상태가 읽히는지 확인한다.

## Tauri 로컬 문서 검증

```sh
pnpm run tauri:dev:annotator
```

1. `specs/029-ma-spec-markdown-preview/spec.md`를 연다.
2. TOC 항목이 올바른 heading으로 이동하는지 확인한다.
3. 같은 디렉터리의 유효한 상대 wikilink가 대상 문서를 여는지 확인한다.
4. 존재하지 않는 target과 directory traversal target에서 현재 문서가 유지되고 오류가 표시되는지 확인한다.
5. 열린 로컬 파일을 외부에서 수정하고 최신 Preview와 watcher 대상이 갱신되는지 확인한다.

## 완료 기준

- [ ] 자동 검증 command가 모두 통과한다.
- [ ] SpecKit 예제 5종을 선택하고 서로 이동할 수 있다.
- [ ] task 본문 summary와 TOC count가 원문과 일치한다.
- [ ] 일반 bullet/code의 task 유사 문구가 task로 표시되지 않는다.
- [ ] local wikilink의 성공·누락·차단 경로가 계약대로 동작한다.
- [ ] Mermaid 오류가 나머지 문서를 숨기지 않는다.
- [ ] HTML5 주석은 숨고 code 내부 주석 문자열과 주석 밖 본문은 보존된다.

## 자동 검증 기록

2026-07-12 구현 검증에서 core 44개, shared React 41개, MA 17개, AW 258개 테스트와 각 TypeScript type check가 통과했다. MA Vite build, Storybook build와 Tauri `cargo check`도 통과했다. 수동 browser/Tauri 시나리오는 release 전 완료 기준 목록으로 유지한다.

# Contract: Markdown Preview and Navigation

## Parser Input Contract

- 입력은 UTF-8 Markdown 문자열이며 문서 순서를 보존한다.
- heading H1~H3는 TOC entry를 생성하고 H4~H6는 본문에만 표시한다.
- GFM `- [ ]`는 open task, `- [x]`와 `- [X]`는 completed task다.
- 일반 bullet과 fenced/inline code 내부의 task 유사 문자열은 task가 아니다.
- `[[target]]`과 `[[target | label]]`만 wikilink다. target/label 주변 공백은 제거한다.
- 비어 있거나 닫히지 않은 wikilink는 일반 텍스트이며 navigation target을 만들지 않는다.
- 한 줄 또는 여러 줄의 완성된 `<!-- ... -->` HTML5 주석은 block parsing 전에 숨기되 내부 개행을 보존하여 source line anchor가 바뀌지 않게 한다.
- inline code와 fenced code 안의 `<!-- ... -->` 문자열은 원본 code로 보존하고, 닫히지 않은 주석은 이후 본문을 숨기지 않도록 일반 텍스트로 유지한다.

## TOC Output Contract

```ts
type TaskSummary = { completed: number; open: number }

type TocEntry = {
  blockId: string
  level: 1 | 2 | 3
  text: string
  startLine: number
  taskSummary?: TaskSummary
}
```

- entry 순서는 source heading 순서와 같다.
- `blockId`는 중복 제목에서도 고유하다.
- `taskSummary`는 task가 하나 이상인 H1 entry에만 존재한다.
- H2/H3 task는 가장 가까운 선행 H1 summary에 포함된다.
- 첫 H1 이전 task는 Preview preamble summary에는 포함하지만 TOC entry에는 포함하지 않는다.

## Renderer Contract

- 완료와 미완료 task는 서로 다른 icon과 accessible text를 제공하며 색상만으로 구분하지 않는다.
- task는 Preview에서 toggle할 수 없다.
- H1 chapter summary는 H1 바로 다음에 표시한다. task 없는 chapter에는 표시하지 않는다.
- TOC H1 task summary는 본문 chapter summary와 동일한 개수를 표시한다.
- 긴 task, nested task, link, inline code는 본문 내용과 정렬을 보존한다.
- 개별 Mermaid 렌더 실패는 해당 block의 오류 안내로 제한하고 다른 block은 유지한다.

## Navigation Contract

### TOC selection

`onEntrySelect(entry)`는 `entry.blockId`에 해당하는 Preview block을 찾아 scroll하고 식별 가능한 focus/highlight를 제공한다.

### Wikilink activation

1. renderer는 wikilink를 `./target.md` 형태의 anchor로 출력한다.
2. MA는 click 또는 keyboard activation을 가로채 현재 문서의 디렉터리를 기준으로 target을 정규화한다.
3. target이 허용 root 내부의 `.md` 파일인지 검증한다.
4. 읽기에 성공하면 document, TOC, annotation/selection, watcher target과 status를 새 문서 기준으로 교체한다.
5. target이 invalid/blocked/missing/unreadable이면 현재 문서를 유지하고 원인과 target을 알린다.
6. 문서 표시만으로 navigation이나 preload를 수행하지 않는다.

예제 문서에서 대상 file name과 일치하는 `ExampleMarkdownDocument`가 있으면 filesystem 대신 해당 raw fixture를 로드한다.

## Example Catalog Contract

다음 id/file 쌍을 예제 목록에서 제공한다.

| ID | File | Artifact |
|---|---|---|
| `speckit-spec` | `speckit-spec.md` | Feature specification |
| `speckit-plan` | `speckit-plan.md` | Implementation plan |
| `speckit-data-model` | `speckit-data-model.md` | Data model |
| `speckit-tasks` | `speckit-tasks.md` | Tasks |
| `speckit-checklist` | `speckit-checklist.md` | Requirements checklist |

각 entry는 비어 있지 않은 title/description/markdownText를 가지며, 예제 path는 reload watcher 대상에서 제외한다.

## Verification Contract

- pure parser/TOC 집계: fixture unit tests
- React renderer/TOC: static markup과 accessibility assertions
- MA navigation/lifecycle: page/feature tests
- reusable UI states: `Molecules/MarkdownViewer`, `Molecules/MarkdownToc` Storybook
- consumers: core, React, MA, AW의 `check-types`와 `test`; MA Vite/Storybook build

## AW Speckit Preview Integration Contract

- Speckit Preview는 현재 문서 blocks를 `MarkdownViewer`에 전달하면서 annotation maps와 block/inline action callbacks를 함께 연결한다.
- 본문 selection은 현재 Preview container 안에서만 anchor로 변환하고 selection delete/note toolbar를 제공한다.
- annotation 생성·편집·삭제는 선택 Speckit 문서 경로의 배열만 변경한다.
- annotation 목록과 agent prompt는 현재 선택 문서의 annotation만 사용한다.
- agent prompt 전송 callback이 없으면 Send action을 비활성화한다.
- 문서 전환 시 열린 dialog, editing id, selection anchors와 highlight를 초기화한다.
- `extractTocEntries(blocks)` 결과를 `MarkdownPreviewToc`에 전달하고 선택한 `blockId`를 Speckit Preview container 안에서 scroll한다.
- task가 포함된 H1 TOC entry는 공용 `taskSummary`의 completed/open 개수를 그대로 표시한다.
- 일반 Markdown panel과 Speckit panel은 같은 AW app-local annotation workspace 계약을 사용하며 공용 package에 worktree 또는 agent callback 의존성을 추가하지 않는다.

### AW integration verification

- 동일 문서에서 block/selection annotation 생성·편집·삭제
- 두 Speckit 문서 사이 annotation 격리와 복귀 시 보존
- 문서 전환 시 selection/dialog 초기화
- annotation prompt 내용과 Send callback
- H1~H3, 중복 제목, H1 task count와 TOC scroll target

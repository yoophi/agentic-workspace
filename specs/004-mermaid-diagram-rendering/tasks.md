# Tasks: Mermaid Diagram Rendering

**입력**: `specs/004-mermaid-diagram-rendering/`의 설계 문서

**전제 조건**: plan.md, spec.md, research.md, data-model.md, contracts/markdown-viewer-mermaid.md, quickstart.md

**테스트**: 필수. 이 기능은 `packages/*`의 공유 parser/viewer 동작을 변경하므로 constitution이 요구하는 단위 테스트, fixture 테스트, 소비 앱 검증 태스크를 포함한다.

**구성**: 각 user story를 독립적으로 구현하고 검증할 수 있도록 태스크를 user story 단위로 묶는다.

## Phase 1: Setup (Shared Infrastructure)

**목적**: 기능 구현 전에 dependency와 package 경계를 준비한다.

- [X] T001 Mermaid renderer dependency를 `packages/markdown-annotation-react/package.json`에 추가하고 `pnpm-lock.yaml`을 갱신한다.
- [X] T002 [P] 현재 Markdown viewer export와 앱 adapter를 `packages/markdown-annotation-react/src/index.ts`, `apps/markdown-annotator/src/shared/ui/markdown-viewer-components.tsx`, `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-viewer-components.tsx`에서 확인한다.

---

## Phase 2: Foundational (Blocking Prerequisites)

**목적**: 모든 user story보다 먼저 완료되어야 하는 공유 core model과 감지 동작을 준비한다.

**중요**: 이 phase가 끝나기 전에는 user story 구현을 시작하지 않는다.

- [X] T003 [P] 선택적 Mermaid metadata type을 `packages/markdown-annotation-core/src/types/markdown-block.ts`에 추가한다.
- [X] T004 [P] language marker와 priority start token 감지를 위한 실패 fixture 테스트를 `packages/markdown-annotation-core/src/mermaid/detect-mermaid-block.test.ts`에 작성한다.
- [X] T005 priority start token 감지 helper를 `packages/markdown-annotation-core/src/mermaid/detect-mermaid-block.ts`에 구현한다.
- [X] T006 Mermaid 감지 helper와 type을 `packages/markdown-annotation-core/src/index.ts` 및 `packages/markdown-annotation-core/src/types/index.ts`에서 export한다.

**Checkpoint**: foundation 준비 완료 - user story 구현을 시작할 수 있다.

---

## Phase 3: User Story 1 - Mermaid blocks render as diagrams (Priority: P1) MVP

**목표**: Mermaid fenced block과 priority Mermaid token으로 시작하는 fenced block은 diagram으로 렌더링하고, 일반 code block은 기존 방식으로 유지한다.

**독립 검증**: 유효한 Mermaid block과 일반 code block이 함께 있는 Markdown을 열거나 렌더링했을 때 Mermaid block만 diagram으로 표시되는지 확인한다.

### Tests for User Story 1

- [X] T007 [P] [US1] `mermaid` language marker, priority start token, non-Mermaid code block parser fixture 테스트를 `packages/markdown-annotation-core/src/parse/parse-markdown-to-blocks.test.ts`에 추가한다.
- [X] T008 [P] [US1] 유효한 diagram 렌더링과 일반 code block 보존을 검증하는 mocked Mermaid shared viewer 테스트를 `packages/markdown-annotation-react/src/MermaidDiagram.test.tsx`에 추가한다.

### Implementation for User Story 1

- [X] T009 [US1] `type: "code"`를 유지하면서 Mermaid metadata를 `packages/markdown-annotation-core/src/parse/parse-markdown-to-blocks.ts`에서 붙인다.
- [X] T010 [US1] `startOnLoad: false`와 strict security 기본값을 사용하는 lazy Mermaid rendering component를 `packages/markdown-annotation-react/src/MermaidDiagram.tsx`에 생성한다.
- [X] T011 [US1] Mermaid code block을 새 diagram component로 렌더링하도록 `packages/markdown-annotation-react/src/MarkdownViewer.tsx`를 수정한다.
- [X] T012 [US1] contained diagram styling과 일반 code block 보존 style을 `packages/markdown-annotation-react/src/styles.css`에 추가한다.
- [X] T013 [P] [US1] Mermaid 성공 렌더링 Storybook state를 `apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx`에 추가한다.

**Checkpoint**: User Story 1을 독립적으로 동작 및 검증할 수 있다.

---

## Phase 4: User Story 2 - Failed Mermaid rendering remains inspectable (Priority: P2)

**목표**: Mermaid 렌더링 실패를 해당 block에만 격리하고, 원본 source와 읽을 수 있는 실패 이유를 표시한다.

**독립 검증**: 잘못된 Mermaid 문법이 포함된 Markdown을 렌더링했을 때 affected block에만 fallback이 표시되고, 원본 source와 실패 이유가 보이며, 나머지 문서가 계속 읽히는지 확인한다.

### Tests for User Story 2

- [X] T014 [P] [US2] empty source, syntax/parse failure, renderer/runtime failure category를 검증하는 shared viewer 테스트를 `packages/markdown-annotation-react/src/MermaidDiagram.test.tsx`에 추가한다.
- [X] T015 [P] [US2] source 표시와 block-local isolation을 검증하는 contract 지향 fallback assertion을 `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`에 추가한다.

### Implementation for User Story 2

- [X] T016 [US2] failure category와 읽을 수 있는 failure reason mapping을 `packages/markdown-annotation-react/src/MermaidDiagram.tsx`에 추가한다.
- [X] T017 [US2] failure reason과 원본 Mermaid source를 보여주는 block-local fallback panel을 `packages/markdown-annotation-react/src/MermaidDiagram.tsx`에 추가한다.
- [X] T018 [US2] fallback, source panel, 긴 error text containment style을 `packages/markdown-annotation-react/src/styles.css`에 추가한다.
- [X] T019 [P] [US2] Mermaid 실패 fallback Storybook state를 `apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx`에 추가한다.

**Checkpoint**: User Story 1과 2를 각각 독립적으로 동작 및 검증할 수 있다.

---

## Phase 5: User Story 3 - Annotation and reload workflows remain intact (Priority: P3)

**목표**: 기존 annotation, text selection, 큰 diagram containment, auto reload 흐름이 Mermaid diagram이 포함된 문서에서도 유지된다.

**독립 검증**: Mermaid diagram과 주변 텍스트가 있는 Markdown 문서에서 selection/annotation이 계속 동작하고, Mermaid source 변경 후 auto reload로 diagram이 갱신되는지 확인한다.

### Tests for User Story 3

- [X] T020 [P] [US3] Mermaid block의 block shell data attribute와 block action을 검증하는 viewer regression 테스트를 `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`에 추가한다.
- [X] T021 [P] [US3] Mermaid source 갱신을 다루는 auto reload integration coverage를 `apps/markdown-annotator/src/pages/annotator/annotator-auto-reload.test.tsx`에 확장한다.

### Implementation for User Story 3

- [X] T022 [US3] Mermaid block이 `data-block-id`, line range attribute, note control, block action control을 유지하도록 `packages/markdown-annotation-react/src/MarkdownViewer.tsx`를 확인 및 수정한다.
- [X] T023 [US3] block id와 source hash를 기준으로 render state를 keying하여 source 변경 시 diagram이 다시 렌더링되도록 `packages/markdown-annotation-react/src/MermaidDiagram.tsx`를 수정한다.
- [X] T024 [US3] 큰 diagram overflow containment를 `packages/markdown-annotation-react/src/styles.css`에 추가한다.
- [X] T025 [P] [US3] 큰 Mermaid diagram Storybook state를 `apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx`에 추가한다.
- [X] T026 [P] [US3] agentic workbench Markdown viewer adapter에 app-shell coupling 변경이 필요 없는지 `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-viewer-components.tsx`에서 검증한다.

**Checkpoint**: 모든 user story를 독립적으로 동작 및 검증할 수 있다.

---

## Phase 6: Polish & Cross-Cutting Concerns

**목적**: 검증, 정리, cross-app safety check를 수행한다.

- [X] T027 [P] `pnpm --filter @yoophi/markdown-annotation-core test`로 shared core 테스트를 실행한다.
- [X] T028 [P] `pnpm --filter @yoophi/markdown-annotation-react test` 및 `pnpm --filter @yoophi/markdown-annotation-react check-types`로 shared React package 테스트와 타입 검사를 실행한다.
- [X] T029 [P] `pnpm --filter @yoophi/markdown-annotator test`, `pnpm --filter @yoophi/markdown-annotator check-types`, `pnpm --filter @yoophi/markdown-annotator build-storybook`으로 markdown annotator 테스트, 타입 검사, Storybook build를 실행한다.
- [X] T030 [P] `pnpm --filter @yoophi/agentic-workbench test` 및 `pnpm --filter @yoophi/agentic-workbench check-types`로 agentic workbench 소비 앱 테스트와 타입 검사를 실행한다.
- [X] T031 `apps/markdown-annotator/src`와 `apps/agentic-workbench/src`를 검색하여 app-to-app import가 추가되지 않았는지 검증한다.
- [X] T032 manual smoke scenario를 `specs/004-mermaid-diagram-rendering/quickstart.md` 기준으로 검증한다.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: dependency 없음 - 즉시 시작 가능.
- **Foundational (Phase 2)**: Setup 완료 후 진행 - 모든 user story를 block한다.
- **User Stories (Phase 3+)**: Foundational 완료 후 진행.
- **Polish (Phase 6)**: 원하는 user story 구현이 완료된 뒤 진행.

### User Story Dependencies

- **User Story 1 (P1)**: Phase 2 이후 시작 가능하며 MVP 범위다.
- **User Story 2 (P2)**: Phase 2 이후 시작 가능하다. US1과 병행 또는 이후 진행할 수 있지만 최종 fallback UI는 US1의 `MermaidDiagram.tsx`에 의존한다.
- **User Story 3 (P3)**: Phase 2 이후 시작 가능하다. reload 및 annotation regression 확인은 US1/US2가 수정한 shared viewer 경로에 의존한다.

### Within Each User Story

- constitution-required test를 먼저 작성하고 구현 전 실패하는지 확인한다.
- shared core 감지를 shared React 렌더링보다 먼저 구현한다.
- rendering component를 app Storybook state보다 먼저 구현한다.
- shared package 동작이 준비된 뒤 Storybook visual state를 추가한다.

## Parallel Opportunities

- T002는 T001과 병렬 진행 가능하다.
- T003과 T004는 foundational phase에서 병렬 진행 가능하다.
- T007과 T008은 US1에서 병렬 진행 가능하다.
- T014와 T015는 US2에서 병렬 진행 가능하다.
- T020과 T021은 US3에서 병렬 진행 가능하다.
- T027부터 T030까지는 구현 완료 후 병렬 실행 가능하다.

## Parallel Example: User Story 1

```bash
Task: "T007 [P] [US1] packages/markdown-annotation-core/src/parse/parse-markdown-to-blocks.test.ts에 Mermaid 감지 parser fixture 테스트 추가"
Task: "T008 [P] [US1] packages/markdown-annotation-react/src/MermaidDiagram.test.tsx에 mocked Mermaid rendering shared viewer 테스트 추가"
Task: "T013 [P] [US1] apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx에 Mermaid 성공 Storybook state 추가"
```

## Parallel Example: User Story 2

```bash
Task: "T014 [P] [US2] packages/markdown-annotation-react/src/MermaidDiagram.test.tsx에 failure category 테스트 추가"
Task: "T015 [P] [US2] packages/markdown-annotation-react/src/MarkdownViewer.test.tsx에 fallback isolation 테스트 추가"
Task: "T019 [P] [US2] apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx에 Mermaid 실패 fallback Storybook state 추가"
```

## Parallel Example: User Story 3

```bash
Task: "T020 [P] [US3] packages/markdown-annotation-react/src/MarkdownViewer.test.tsx에 block shell regression 테스트 추가"
Task: "T021 [P] [US3] apps/markdown-annotator/src/pages/annotator/annotator-auto-reload.test.tsx에 auto reload integration coverage 확장"
Task: "T025 [P] [US3] apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx에 큰 Mermaid diagram Storybook state 추가"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 setup을 완료한다.
2. Phase 2 shared core detection을 완료한다.
3. Phase 3 Mermaid 성공 렌더링을 완료한다.
4. `pnpm --filter @yoophi/markdown-annotation-core test`, `pnpm --filter @yoophi/markdown-annotation-react test`, MarkdownViewer Storybook 성공 state로 검증하고 멈춘다.

### Incremental Delivery

1. US1을 추가하여 유효한 Mermaid diagram을 렌더링하고 일반 code block은 유지한다.
2. US2를 추가하여 block-local failure reason과 source inspection을 제공한다.
3. US3를 추가하여 annotation, reload, large diagram 동작을 강화한다.
4. 선택한 story가 모두 끝나면 전체 quickstart validation을 실행한다.

### Parallel Team Strategy

1. 한 개발자는 `packages/markdown-annotation-core`의 shared core 감지를 담당한다.
2. 한 개발자는 `packages/markdown-annotation-react`의 shared React rendering/fallback을 담당한다.
3. 한 개발자는 `apps/markdown-annotator`와 `apps/agentic-workbench`의 Storybook 및 app consumer 검증을 담당한다.

## Notes

- 모든 user-story task는 `[US1]`, `[US2]`, `[US3]` label을 포함한다.
- `[P]` task는 서로 다른 파일을 다루거나 검증 command라서 미완료 동일 파일 수정에 의존하지 않고 병렬 진행할 수 있다.
- shared package 변경은 package-level verification과 consuming app verification을 모두 요구한다.

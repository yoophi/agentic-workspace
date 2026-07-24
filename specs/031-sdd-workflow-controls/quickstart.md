# Quickstart: SDD 워크플로 단계 표시 및 제어

## Prerequisites

- Repository root: `agentic-workspace`
- `.specify/feature.json` points to `specs/031-sdd-workflow-controls` for the active-feature scenario
- AW dependencies are installed

## Validation Commands

```bash
pnpm --filter agentic-workbench check-types
pnpm --filter agentic-workbench test
```

No Rust command is required unless the implementation changes the existing worktree-file backend contract.

## Manual Validation Scenarios

### Scenario 1: Active feature highlight and stage state

1. Open a Worktree Session with several directories under `specs/`.
2. Set `.specify/feature.json` to a valid `feature_directory` matching one listed feature.
3. Open the Speckit tab.

**Expected**:

- The matching feature row has a visible `현재 작업 중` label and highlight.
- The control shows Specify, Plan, Tasks, Implement in order.
- Artifact presence and tasks progress produce the expected current/complete/pending state.

### Scenario 2: Confirmation and SDD request

1. Use a feature with `spec.md` but no `plan.md`.
2. Select Plan.
3. Review the confirmation and confirm it.
4. Repeat with an already existing stage and choose re-run.

**Expected**:

- Plan requires a spec review confirmation before it is sent.
- Re-run warns that existing artifacts may be affected.
- A confirmed action targets the active feature and current worktree session.
- A disabled later stage explains its missing prerequisite.

### Scenario 3: Missing or invalid active pointer draft

1. Remove `.specify/feature.json`, use malformed JSON, or point it at a nonexistent feature.
2. Open the Speckit tab and choose the SDD start guidance action.

**Expected**:

- No arbitrary Speckit feature is highlighted as active.
- The agent prompt textarea receives an editable initial SDD draft.
- No agent run starts, no queue item is created, and prompt history remains unchanged until the user explicitly sends it.

### Scenario 4: External change refresh

1. Keep the Speckit tab open with a valid active pointer.
2. Change `.specify/feature.json` to a different valid feature, then change or remove its artifacts.
3. Wait for the watcher or select refresh.

**Expected**:

- Highlight and stage state reflect the new pointer and artifacts.
- A stale pointer reports unavailable instead of highlighting the previous feature.

### Scenario 5: Tasks Kanban and Needed Tasks

1. 완료/미완료 checkbox와 서로 다른 heading section이 있는 `tasks.md`를 선택한다.
2. Kanban에서 진행할 작업만 및 완료된 작업만 filter를 각각 선택한다.
3. 작업 필요 보기로 전환한다.

**Expected**:

- Kanban 열과 filter는 해당 완료 상태 작업만 표시한다.
- 작업 필요 보기에는 미완료 작업이 있는 section의 heading·문맥·미완료 작업만 표시된다.
- 완료 task와 완료-only section은 표시되지 않으며 원본 파일은 바뀌지 않는다.

## Storybook Validation

Add/update AW organism stories for:

- valid active feature with stage controls and highlighted row
- valid active feature whose tasks are complete
- missing, malformed, and stale active pointer states
- disabled prerequisite and review/re-run confirmation states
- initial prompt draft injection callback

## Focused Test Fixtures

- valid pointer: `{ "feature_directory": "specs/001-alpha" }`
- malformed JSON, missing `feature_directory`, non-string value
- absolute path, `../` traversal, `specs/001-alpha/../002-beta`, outside `specs/`
- valid pointer not present in `SpeckitFeature[]`
- feature documents for spec-only, spec+plan, spec+plan+tasks, and completed tasks
- send and draft `AgentPromptRequest` on idle and running agent panel states

## Validation Log

- 2026-07-24: `pnpm --filter agentic-workbench check-types` passed.
- 2026-07-24: `pnpm --filter agentic-workbench test` passed with 45 test files and 273 tests.
- 2026-07-24: Desktop GUI manual scenarios were not run in this non-interactive session; execute Scenarios 1–4 in a running AW desktop session before release.

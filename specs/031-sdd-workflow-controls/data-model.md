# Data Model: SDD 워크플로 단계 표시 및 제어

## ActiveFeaturePointer

`.specify/feature.json`에서 읽은 활성 기능 추적 정보다.

**Fields**:

- `status`: `loading | active | unavailable | error`
- `featurePath`: 검증된 경우의 root-relative `specs/<feature>` 경로, 그 외에는 `null`
- `reason`: unavailable/error 상태의 짧은 사용자 표시 사유

**Validation rules**:

- JSON 객체의 string `feature_directory`만 허용한다.
- 값은 relative path여야 하고 `..` segment 또는 absolute path를 포함할 수 없다.
- 값은 `specs/` 아래 하나의 feature directory여야 한다.
- 유효한 값도 현재 `SpeckitFeature.relativePath`와 일치할 때만 `active`다.
- 선택 문서, 정렬, 목록의 다른 feature로 대체하지 않는다.

## SddStage

`specify | plan | tasks | implement` 중 하나다.

## SddStageState

하나의 stage를 사용자에게 표시하고 실행 여부를 결정하는 순수 모델이다.

**Fields**:

- `stage`: `SddStage`
- `status`: `complete | current | pending | unavailable`
- `artifactPath`: 완료 판별 근거인 산출물 상대 경로 또는 `null`
- `canStart`: 사용자 action이 가능한지
- `blockedReason`: 실행 불가 사유 또는 `null`
- `requiresConfirmation`: 검토 게이트 또는 재실행 확인이 필요한지

**Transitions**:

- 활성 pointer unavailable이면 네 단계 모두 unavailable이며 Specify만 초기 prompt 안내를 제공한다.
- `spec.md`가 없으면 specify는 current, 이후 단계는 pending/unavailable이다.
- `spec.md`만 있으면 specify는 complete, plan은 current이다.
- `plan.md`까지 있으면 tasks는 current이다.
- `tasks.md`까지 있으면 implement는 current이다. tasks progress가 complete이면 implement는 complete 안내 상태가 될 수 있지만 재실행은 확인 후 가능하다.
- 이전 산출물이 이미 있는 stage를 다시 시작할 때 `requiresConfirmation`은 true다.
- Plan 및 Tasks 시작은 각각 spec/plan review confirmation을 요구한다.

## SddActionRequest

사용자가 컨트롤에서 요청한 작업이다.

**Fields**:

- `stage`: 요청 stage
- `featurePath`: 활성 feature path 또는 `null`
- `delivery`: `send | draft`
- `prompt`: 사용자에게 표시·전달할 SDD 명령 텍스트
- `confirmation`: `none | reviewSpec | reviewPlan | rerun`

**Validation rules**:

- `send`는 검증된 active feature와 필요한 confirmation 완료 후에만 생성한다.
- `draft`는 active pointer unavailable일 때의 초기 SDD 안내에만 사용한다.
- prompt는 빈 문자열일 수 없고 자동으로 실행되지 않는 draft는 사용자 편집 가능해야 한다.

## AgentPromptRequest Extension

현재 workspace가 agent run area로 전달하는 prompt request다.

**Fields**:

- `id`: 한 번만 처리할 request identity
- `text`: prompt text
- `delivery`: `send | draft` (기존 요청은 `send`로 호환)

**Behavior**:

- `send`: agent run panel이 현재 동작처럼 idle이면 run을 시작하고, running이면 queue에 넣는다.
- `draft`: agent run panel이 prompt textarea만 갱신한다. run 시작, queue 추가, history 기록은 하지 않는다.

## Relationships

- 하나의 `ActiveFeaturePointer`는 최대 하나의 `SpeckitFeature`를 가리킨다.
- 하나의 활성 `SpeckitFeature`는 4개의 `SddStageState`를 생성한다.
- `SddActionRequest`는 활성 feature와 현재 worktree session을 통해 하나의 `AgentPromptRequest`가 된다.
- `SpeckitFilesPanel`은 `activeFeaturePath`와 feature `relativePath`가 같을 때 해당 feature를 highlight한다.

## TaskKanbanItem and NeededTaskSection

**TaskKanbanItem**은 `tasks.md`의 checkbox task를 나타내며 `id`, `text`, `completed`, `sourceLine`, `sectionId`를 가진다.

**NeededTaskSection**은 `id`, `heading`, `contextLines`, `incompleteTasks`를 가진다. `incompleteTasks`가 비어 있는 section은 작업 필요 보기에 존재할 수 없다.

**Validation rules**:

- 완료 checkbox는 `NeededTaskSection`에 포함하지 않는다.
- heading 없는 task는 synthetic section으로 묶고 바로 인접한 non-task line만 context로 유지한다.
- markdown 원문과 checkbox 상태는 수정하지 않는다.

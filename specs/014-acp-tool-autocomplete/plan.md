# 구현 계획: ACP Tool List 기반 Prompt Command 자동완성

**브랜치**: `[]` | **작성일**: 2026-07-06 | **명세**: [spec.md](./spec.md)

**입력**: `/specs/014-acp-tool-autocomplete/spec.md`의 기능 명세

**참고**: 이 문서는 `/speckit-plan` 명령으로 생성된 구현 계획이며, 실행 흐름은 `.specify/templates/plan-template.md`를 따른다.

## 요약

Agentic Workbench의 prompt 입력 영역에서 `$` 또는 `/` 토큰을 입력하면 현재 agent session 범위에서 사용할 수 있는 tool/command 후보를 보여주고, 키보드 또는 포인터 선택으로 prompt draft에 command token을 삽입한다. 구현은 AW 내부 기능으로 시작하며, 후보 모델과 조회 상태는 `entities/agent-run`, 자동완성 상호작용과 UI는 `features/agent-run`, 재사용 가능한 prompt input primitive 확장은 `components/ui` 경계에서 다룬다. 후보 선택은 prompt draft만 수정하고 agent 실행, tool 호출, permission 응답은 시작하지 않는다.

## 기술 맥락

**언어/버전**: TypeScript/React 프론트엔드, Rust Tauri 백엔드

**주요 의존성**: Tauri invoke/event bridge, React Query, 기존 shadcn 스타일 UI primitive(`PromptInput`, `Popover`, `Command`, `Tooltip`), 기존 ACP runner/session registry

**저장소**: 자동완성 상태는 저장하지 않는다. 후보 데이터는 session-scoped runtime state이며 이 기능에서 persistence 대상이 아니다.

**테스트**: TypeScript model/UI는 Vitest로 검증한다. prompt composer 상태는 Storybook story/interaction으로 검증한다. 후보 조회가 백엔드 지원을 필요로 하면 backend contract와 session scope 변경은 Cargo test/check로 검증한다.

**대상 플랫폼**: Agentic Workbench 데스크톱 앱

**프로젝트 유형**: pnpm/Turbo monorepo 안의 Rust backend 기반 Tauri 데스크톱 앱

**성능 목표**: 후보가 50개 이상이어도 눈에 띄는 입력 지연 없이 검색 가능해야 한다. 일반 prompt 작성 중 prefix filtering과 keyboard movement는 즉각적으로 느껴져야 한다.

**제약**: 후보 선택은 prompt를 submit하거나 agent tool을 호출하거나 permission을 승인하거나 파일을 변경하면 안 된다. 자동완성은 기존 prompt history navigation, queue shortcut, saved prompt insertion, Ralph loop input mode, running/idle send behavior를 깨면 안 된다.

**규모/범위**: AW prompt composer 한 곳, 현재 run/session scoped candidate source, `$`와 `/` prefix, 이름과 짧은 설명이 있는 후보 표시, keyboard/pointer selection, empty/loading/no-match 상태를 포함한다.

## Constitution 점검

*게이트: Phase 0 research 전에 통과해야 하며, Phase 1 design 후 다시 점검한다.*

- **Monorepo Boundary First**: PASS. 작업 범위는 `apps/agentic-workbench`로 제한하며 app-to-app import나 새 shared package를 계획하지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS. 후보 type/API는 `apps/agentic-workbench/src/entities/agent-run`에 둔다. prompt autocomplete behavior는 `apps/agentic-workbench/src/features/agent-run`에 둔다. 재사용 가능한 low-level input/popover 조합은 primitive contract 변경이 필요한 경우에만 `apps/agentic-workbench/src/components/ui`에 둔다.
- **Hexagonal Tauri Backend Architecture**: PASS. 백엔드 지원이 필요하면 session-scoped candidate lookup은 domain/application/ports로 모델링하고 inbound Tauri command는 application service에 위임한다. ACP adapter detail은 infrastructure에 남긴다.
- **Shared Core Before Shared UI**: PASS. cross-app sharing은 제안하지 않는다. matching/token helper는 두 번째 consumer가 생기기 전까지 local helper로 유지한다.
- **Atomic Cross-App Verification**: N/A. `packages/*` 또는 `crates/*` 변경을 계획하지 않는다.
- **Documentation and Storybook**: PASS. loading, empty, many-candidate, keyboard-selected, long-description 자동완성 상태를 Storybook에 추가해야 한다. 구현 중 이 계획을 넘어서는 architecture decision이 나오지 않는 한 별도 `docs/*.md`는 필요하지 않다.
- **Testing and Safety**: PASS. 순수 token parsing/filtering helper는 unit test를 둔다. UI test는 keyboard conflict를 검증한다. candidate retrieval이 Tauri boundary를 건너면 backend test는 run/session owner scope를 검증한다.

## 프로젝트 구조

### 문서 구조(이 기능)

```text
specs/014-acp-tool-autocomplete/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── prompt-autocomplete-ui.md
│   └── session-tool-candidates.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### 소스 코드 구조(repository root)

```text
apps/agentic-workbench/src/
├── components/ui/
│   └── prompt-input.tsx                 # key handling/ref contract 변경이 필요할 때만 primitive 확장
├── entities/agent-run/
│   ├── api/
│   │   ├── agent-run-repository.ts       # backend가 필요할 경우 session-scoped candidate query adapter
│   │   └── query-options.ts              # async query를 쓸 경우 candidate query option/cache key
│   └── model/
│       ├── types.ts                      # tool/command candidate type
│       ├── prompt-autocomplete.ts        # trigger parsing, filtering, insertion helper
│       └── prompt-autocomplete.test.ts
├── features/agent-run/
│   ├── model/
│   │   └── prompt-autocomplete-state.ts  # UI state가 커질 경우 selection/open state reducer
│   └── ui/
│       ├── agent-run-panel.tsx           # prompt composer integration
│       ├── prompt-command-autocomplete.tsx
│       └── prompt-command-autocomplete.test.tsx
└── stories/
    └── organisms.stories.tsx             # prompt autocomplete Storybook states

apps/agentic-workbench/src-tauri/src/
├── domain/
│   └── agent_tool_candidate.rs           # backend candidate lookup이 필요할 경우에만 추가
├── application/
│   └── agent_tool_candidate_service.rs   # session/run scoped candidate use case
├── inbound/
│   └── tauri_commands.rs                 # thin command delegation
├── infrastructure/
│   └── acp/runner.rs                     # 가능한 경우 capture된 ACP/tool candidate metadata 노출
└── ports/
    └── session_registry.rs               # 필요 시 run owner/session lookup 확장
```

**구조 결정**: AW-local 기능으로 구현한다. token parsing과 filtering은 domain-specific이지만 UI-independent이므로 `entities/agent-run/model`에 둔다. popover/list 상호작용은 prompt composer behavior에 결합되어 있으므로 `features/agent-run/ui`에 둔다. backend 변경은 candidate source에 따라 조건부로 수행한다. 기존 frontend가 session tool metadata를 받을 수 없다면 hexagonal backend layer를 통해 session-scoped Tauri command를 추가한다.

## Phase 0: Research

[research.md](./research.md)를 참조한다.

주요 결정:

- v1에서는 `$`와 `/`를 동일한 autocomplete prefix로 사용하되, 삽입 token에는 사용자가 입력한 prefix를 보존한다.
- 자동완성은 prompt draft editing으로만 취급하며, 선택은 실행을 유발하지 않는다.
- 후보는 session scoped runtime data로 유지하고 persistence하지 않는다.
- keyboard conflict behavior를 전체 panel 렌더링 없이 테스트할 수 있도록 local headless parser/filter helper를 둔다.
- ACP runner가 initialized session tool metadata를 노출할 수 있을 때만 backend candidate lookup을 추가한다. 그렇지 않으면 prompt typing을 막지 않는 empty/loading state를 지원하는 candidate provider abstraction부터 시작한다.

## Phase 1: Design & Contracts

다음 산출물을 참조한다.

- [data-model.md](./data-model.md)
- [contracts/prompt-autocomplete-ui.md](./contracts/prompt-autocomplete-ui.md)
- [contracts/session-tool-candidates.md](./contracts/session-tool-candidates.md)
- [quickstart.md](./quickstart.md)

Agent context update: N/A. 이 repository의 `.specify/scripts/bash/`에는 `.specify` agent-context update script가 없다.

## Constitution 재점검

*Post-design re-check.*

- **Monorepo Boundary First**: PASS. design artifact는 구현 범위를 `apps/agentic-workbench` 아래로 유지한다.
- **Feature-Sliced Frontend Architecture**: PASS. data model은 entity candidate/token helper와 feature UI integration을 분리한다.
- **Hexagonal Tauri Backend Architecture**: PASS. `session-tool-candidates.md`는 inbound command가 application service에 위임하고 infrastructure ACP/session registry adapter를 사용하도록 요구한다.
- **Shared Core Before Shared UI**: PASS. shared UI package를 추가하지 않는다. 순수 helper는 local이며 테스트 가능하다.
- **Atomic Cross-App Verification**: N/A. cross-app package/crate 변경을 계획하지 않는다.
- **Documentation and Storybook**: PASS. quickstart는 reusable prompt autocomplete state에 대한 Storybook state verification을 요구한다.
- **Testing and Safety**: PASS. contract는 selection 시 실행 금지, session scope, keyboard conflict check, persistence 금지를 명시한다.

## Complexity Tracking

constitution 위반 없음.

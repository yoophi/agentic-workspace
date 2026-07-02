# Implementation Plan: Agent 프로필과 환경변수 주입

**Branch**: `feat/agent-env-profiles` | **Date**: 2026-07-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/008-agent-env-profiles/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

ACP agent 실행 설정을 "command 문자열 override"에서 "프로필(이름 + type + command + env)" 모델로 확장한다(GitHub 이슈 #121). 기존 `AgentCommandOverrides`(globalCommand/agentCommands)를 하위 호환으로 유지하면서 프로필 리스트와 공통/프로필별 환경변수를 추가하고, 저장 시 normalization·기본 프로필 seed·최소 1개 활성 불변식을 적용한다. agent 실행 시 프론트가 선택된 프로필의 command/env를 해석해 run request로 전달하고, Rust ACP runner가 child process spawn 시 `.envs(...)`로 주입한다. 세션 시작 UI는 enabled 프로필 목록에서 선택한다.

## Technical Context

**Language/Version**: TypeScript 5.x (React 19, Vite), Rust 2024 edition (Tauri 2)

**Primary Dependencies**: TanStack Query v5, 기존 JSON settings 저장소(`JsonAgentRunSettingsRepository`), ACP runner(`infrastructure/acp/runner.rs`), agent catalog(`ConfigurableAgentCatalog`)

**Storage**: 기존 agent-run settings JSON 파일. override는 가상 working_directory 키(`__app_agent_command_overrides__`) 항목에 저장 — 이 구조를 유지하고 필드만 확장

**Testing**: vitest(모델 normalization/해석 로직), cargo test(도메인 normalization/merge/seed, runner 단위), Storybook(편집기 상태)

**Target Platform**: macOS 데스크톱(Tauri 2), 기존 지원 플랫폼 동일

**Project Type**: desktop-app — `apps/agentic-workbench` 단독 범위(공유 패키지/crate 변경 없음)

**Performance Goals**: 해당 없음(설정 CRUD + spawn 시 env 주입, 성능 민감 경로 아님)

**Constraints**: 기존 저장 데이터(globalCommand/agentCommands) migration 없이 로드·동작 유지(FR-012). env value는 로그·오류 메시지에 노출 금지. PATH 등 기본 실행 환경 불파괴. Tauri command는 위임만(hexagonal 유지)

**Scale/Scope**: 프로필 수 수 개~수십 개, env 항목 수십 개 수준 — 알고리즘 복잡도 무관

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 전 변경이 `apps/agentic-workbench`(src + src-tauri) 내부. 앱 간 import 없음, 공유 패키지/crate 변경 없음.
- **Feature-Sliced Frontend Architecture**: PASS — 프로필/env 순수 로직(normalization, merge, resolve, invariant)은 `features/agent-command-override/model`, 편집 UI는 `features/agent-command-override/ui`, 세션 시작 선택은 `features/agent-run`, 타입과 저장 API는 `entities/agent-run`(model/types.ts, api).
- **Hexagonal Tauri Backend Architecture**: PASS — 도메인 타입 확장은 `domain/agent_run_settings.rs`, normalization·seed·불변식·env merge는 `application/agent_run_settings_service.rs`·`start_agent_run.rs`, spawn 주입은 `infrastructure/acp/runner.rs`. Tauri command 시그니처는 기존 `get/save_agent_run_settings`, `start_agent_run` 유지(payload 필드만 확장).
- **Shared Core Before Shared UI**: N/A — 공유 제안 없음(앱 전용). env 편집기 UI는 앱 로컬로 시작한다.
- **Atomic Cross-App Verification**: N/A — packages/crates 변경 없음. AW 앱 check-types/test/cargo test로 완결.
- **Documentation and Storybook**: PASS — 프로필 편집기(기본/커스텀/disable 차단/빈 목록)와 env 편집기 스토리를 organisms에 추가. 설정 문서 갱신은 tasks에 포함.
- **Testing and Safety**: PASS — 순수 로직(normalization, merge 우선순위, seed, 최소 1개 활성 불변식, legacy 매핑)은 frontend/backend 양쪽 단위 테스트 선행. env value 비노출(오류 메시지에 key만 포함) 규칙을 테스트로 고정. 설정 저장은 기존 repository 경계 유지.

## Project Structure

### Documentation (this feature)

```text
specs/008-agent-env-profiles/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── settings-and-run.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── entities/agent-run/
│   ├── model/types.ts                       # AgentProfile, AgentCommandOverrides 확장,
│   │                                        #   AgentRunRequest에 env 필드
│   └── api/agent-run-repository.ts          # startAgentRun payload에 env 전달
├── features/agent-command-override/
│   ├── model/
│   │   ├── command-overrides.ts             # normalization·legacy 매핑·seed·resolve(env 포함)
│   │   ├── profile-invariants.ts            # 최소 1개 활성 기본 프로필 등 불변식 (신규)
│   │   └── command-override-form.ts         # 프로필/env 폼 상태 helper 확장
│   └── ui/
│       ├── agent-command-override-editor.tsx  # 프로필 리스트 편집기로 확장
│       └── env-var-editor.tsx               # key/value 편집기 (신규)
├── features/agent-run/
│   ├── model/run-panel-state.ts             # resolveRequestAgentCommand → 프로필 기반 해석(env 포함)
│   └── ui/agent-run-panel.tsx               # agent 선택 → enabled 프로필 선택
├── pages/settings/ui/settings-page.tsx      # 편집기 연결(기존 위치)
└── stories/organisms.stories.tsx            # 프로필/env 편집기 상태 스토리

apps/agentic-workbench/src-tauri/src/
├── domain/agent_run_settings.rs             # AgentProfile, overrides에 profiles/global_env 확장
├── domain/run.rs                            # AgentRunRequest에 agent_env 필드
├── application/
│   ├── agent_run_settings_service.rs        # normalization·seed·불변식 검증·env merge 해석
│   └── start_agent_run.rs                   # request의 env를 launcher로 전달
└── infrastructure/acp/runner.rs             # spawn 시 .envs(merged) 주입, PATH 보강과 병합
```

**Structure Decision**: 새 디렉터리 없이 기존 feature/entities/domain 배치를 확장한다. override 저장 위치(가상 settings 키 `__app_agent_command_overrides__`)와 Tauri command 표면(`get/save_agent_run_settings`, `start_agent_run`)은 그대로 두고 payload 스키마만 하위 호환으로 넓힌다.

## Complexity Tracking

> Constitution Check 위반 없음 — 해당 없음.

# Tasks: Agent 프로필과 환경변수 주입

**Input**: Design documents from `/specs/008-agent-env-profiles/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/settings-and-run.md, quickstart.md (GitHub 이슈 #121)

**Tests**: 사용자 여정은 quickstart.md 수동 시나리오로 검증한다. Constitution 필수 테스트(순수 로직: normalization·seed·병합·불변식·해석·secret 비노출)는 생략 불가 — 구현 task 앞에 테스트 task를 배치하고 구현 전 FAIL을 확인한다.

**Organization**: spec.md의 user story(P1 env 주입 / P2 프로필 복수 등록 / P3 안전장치) 단위 phase. 전 범위가 `apps/agentic-workbench` 단독(공유 패키지/crate 변경 없음).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 실행 가능(다른 파일, 미완료 task 의존 없음)
- **[Story]**: US1(env 주입), US2(프로필 복수 등록·선택), US3(기본 프로필 안전장치)

## Path Conventions

- **frontend**: `apps/agentic-workbench/src/{entities,features,pages,stories}`
- **backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure}`

---

## Phase 1: Setup (타입/스키마 확장)

**Purpose**: 양쪽 언어의 데이터 모델을 하위 호환으로 확장한다 (data-model.md, contracts §1·§2).

- [x] T001 [P] TS 타입 확장: `apps/agentic-workbench/src/entities/agent-run/model/types.ts`에 `AgentType`, `AgentProfile`(id/name/agentType/command/env/enabled/builtIn), `AgentCommandOverrides`에 `globalEnv?`/`profiles?` 추가, run request 타입에 `agentEnv?: Record<string, string>` 추가, `AgentCommandSource`에 `profileCommand` 추가
- [x] T002 [P] Rust 타입 확장: `apps/agentic-workbench/src-tauri/src/domain/agent_run_settings.rs`에 `AgentProfile` struct(serde camelCase, `#[serde(default)]`)와 `AgentCommandOverrides`의 `global_env: BTreeMap<String,String>`/`profiles: Vec<AgentProfile>` 필드, `AgentCommandSource::ProfileCommand` variant 추가; `domain/run.rs`의 `AgentRunRequest`에 `agent_env: Option<BTreeMap<String,String>>` 추가(기존 테스트의 struct 리터럴 갱신 포함)

**Checkpoint**: `pnpm --filter agentic-workbench check-types` + `cargo check` 통과, 기존 저장 JSON이 새 타입으로 역직렬화 가능(serde default).

---

## Phase 2: Foundational (normalization·seed·legacy 매핑·불변식)

**Purpose**: 모든 story가 의존하는 순수 로직. 테스트 선행 (research R1·R2·R3·R7, data-model 불변식).

**⚠️ CRITICAL**: 이 phase 완료 전에는 user story 작업을 시작하지 않는다.

- [x] T003 [P] frontend 모델 테스트 먼저(FAIL 확인): `apps/agentic-workbench/src/features/agent-command-override/model/command-overrides.test.ts`에 (a) env normalization — 빈/공백 key 제거·key trim·빈 value 유지, (b) `effectiveProfiles` — 기본 프로필 4종 seed, legacy `agentCommands[type]` → 기본 프로필 command 초기값 매핑, 기존 profiles 보존; `model/profile-invariants.test.ts`(신규)에 (c) 마지막 활성 기본 프로필 disable 불가 판정(`canDisableProfile`), 활성 0개 payload 검출
- [x] T004 [P] backend 테스트 먼저(FAIL 확인): `apps/agentic-workbench/src-tauri/src/application/agent_run_settings_service.rs` 테스트 모듈에 (a) normalization — env key trim/제거, 프로필 name 기본값, 빈 command→None, (b) seed — profiles 비어 있으면 기본 4종 + legacy command 매핑, 기본 프로필 누락 시 복원, (c) 활성 기본 프로필 0개 저장 시 오류(메시지에 env value 미포함), (d) legacy-only 저장 데이터 로드 호환
- [x] T005 frontend 구현: `features/agent-command-override/model/command-overrides.ts`에 env/프로필 normalization·`effectiveProfiles`(seed+legacy 매핑)·기본 프로필 상수(4종, id=agentType) 구현, `model/profile-invariants.ts`(신규)에 `canDisableProfile`/`hasActiveBuiltInProfile` 구현 — T003 통과
- [x] T006 backend 구현: `application/agent_run_settings_service.rs`에 `normalize_command_overrides` 확장(env·프로필 normalization, seed, legacy 매핑)과 저장 시 불변식 검증 오류(`"At least one built-in agent profile must stay enabled."`) 구현 — T004 통과
- [x] T007 Foundational 검증: `pnpm --filter agentic-workbench test` + `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml` 통과, 기존 command-override 테스트 회귀 없음

**Checkpoint**: 프로필 모델의 저장/로드/불변식이 양쪽에서 동일 규칙으로 동작.

---

## Phase 3: User Story 1 - Agent 실행에 환경변수 주입 (Priority: P1) 🎯 MVP

**Goal**: 설정에서 global/프로필 env를 등록하고, 실행 시 병합(프로필 우선)되어 child process에 주입된다 (FR-001~004, research R4·R5·R8).

**Independent Test**: quickstart S1 — 기본 프로필에 env 저장 → 실행 → 프로세스 env 확인, 병합 우선순위 확인, 빈 key 정리 확인. (기본 프로필 id = 기존 agent id라 세션 선택 UI 변경 없이 검증 가능)

### Tests for User Story 1 (constitution-required) ⚠️

- [ ] T008 [P] [US1] frontend 해석 테스트 먼저(FAIL 확인): `command-overrides.test.ts`에 `resolveAgentProfileLaunch` — env 병합(profile > global), command 폴백(profile.command → globalCommand → catalog 기본), source 판정, 미해석 시 null
- [ ] T009 [P] [US1] backend 테스트 먼저(FAIL 확인): `application/agent_run_settings_service.rs`에 env merge 해석 테스트, `infrastructure/acp/runner.rs`에 env 주입용 순수 헬퍼 테스트 — `agent_env`의 PATH 존재 시 `사용자 PATH + ":" + enriched_path()` 결합, 부재 시 enriched만

### Implementation for User Story 1

- [ ] T010 [US1] frontend 해석 구현: `features/agent-command-override/model/command-overrides.ts`에 `resolveAgentProfileLaunch({ profileId, overrides, agents })` 구현(contracts §3) — T008 통과
- [ ] T011 [US1] 실행 요청 연결(frontend): `entities/agent-run/api/agent-run-repository.ts`의 `startAgentRun` payload에 `agentEnv` 전달, `features/agent-run/model/run-panel-state.ts`의 `resolveRequestAgentCommand`를 프로필 기반 해석(command+env 반환)으로 확장 + 기존 단위 테스트 갱신(`run-panel-state` 테스트)
- [ ] T012 [US1] 실행 경로 연결(backend): `application/start_agent_run.rs`가 request의 `agent_env`를 launcher에 전달, `infrastructure/acp/runner.rs` spawn 시 `.envs(merged)` 적용 + PATH 결합 규칙 구현(오류/로그에 env value 미출력) — T009 통과
- [ ] T013 [US1] env 편집기 UI: `features/agent-command-override/ui/env-var-editor.tsx`(신규, key/value 행 추가·삭제·편집) 작성, `ui/agent-command-override-editor.tsx`에 global env 섹션과 기본 프로필별 env 편집을 연결(프로필 리스트 전체 개편은 US2), `pages/settings/ui/settings-page.tsx` 저장 흐름 확인
- [ ] T014 [P] [US1] Storybook: env 편집기 상태(빈 목록/다수 항목/공백 key 입력) 스토리를 `apps/agentic-workbench/src/stories/organisms.stories.tsx`에 추가
- [ ] T015 [US1] US1 검증: quickstart S1 수행(주입·병합·normalization·2분 이내) + `pnpm --filter agentic-workbench check-types && test` + `cargo test`

**Checkpoint**: US1 단독 MVP — env 주입이 명령 문자열 우회 없이 동작.

---

## Phase 4: User Story 2 - 프로필 복수 등록과 세션 시작 선택 (Priority: P2)

**Goal**: 동일 type의 커스텀 프로필을 추가/수정/삭제하고, 세션 시작에서 enabled 프로필을 선택해 그 구성으로 실행한다 (FR-005~007, FR-011, research R5·R6).

**Independent Test**: quickstart S2 — 같은 type 프로필 2개 등록, 목록 표시, 각각 선택 실행, 삭제 반영, command 미지정 폴백.

### Tests for User Story 2 (constitution-required) ⚠️

- [ ] T016 [P] [US2] 폼 모델 테스트 먼저(FAIL 확인): `features/agent-command-override/model/command-override-form.test.ts`에 프로필 추가(UUID id, builtIn=false)/수정/삭제 폼 상태 전이, 저장 payload 변환(normalization 경유), 커스텀만 삭제 가능

### Implementation for User Story 2

- [ ] T017 [US2] 폼 모델 구현: `features/agent-command-override/model/command-override-form.ts`에 프로필 CRUD 폼 상태 helper 확장 — T016 통과
- [ ] T018 [US2] 프로필 편집기 UI: `features/agent-command-override/ui/agent-command-override-editor.tsx`를 프로필 리스트 편집기로 확장 — 프로필 카드(이름/type/command/env-var-editor), 커스텀 추가 버튼(type 선택), 커스텀 삭제 버튼, 기본/커스텀 구분 표시
- [ ] T019 [US2] 세션 시작 프로필 선택: `features/agent-run/ui/agent-run-panel.tsx`의 agent 선택을 enabled `effectiveProfiles` 목록(이름+type 병기)으로 교체, 선택 저장은 profile id(worktree settings `agentId` 재사용), 로드 시 부재/disabled면 첫 enabled 프로필 폴백, 실행·세션 조회는 `profile.agentType` 사용(`listProviderSessions`, ACP 세션 흐름) — `run-panel-state.ts` 폴백 로직 단위 테스트 포함
- [ ] T020 [P] [US2] Storybook: 프로필 편집기(기본+커스텀 혼재, 같은 type 다수) 스토리를 `stories/organisms.stories.tsx`에 추가
- [ ] T021 [US2] US2 검증: quickstart S2 수행 + 자동 테스트(check-types/vitest/cargo test)

**Checkpoint**: US1+US2 — 프로필 기반 실행 구성이 완성.

---

## Phase 5: User Story 3 - 기본 프로필 관리와 안전장치 (Priority: P3)

**Goal**: 기본 프로필 seed·삭제 불가·disable 차단 안내가 UI에 노출된다 (FR-008~010, research R7). 판정 로직은 Phase 2에서 구현 완료 — 이 phase는 UI 노출과 종단 확인.

**Independent Test**: quickstart S3 — 새 환경 기본 4개, 삭제 버튼 없음, 마지막 활성 기본 프로필 disable 차단+안내, disabled 프로필 목록 제외, 파일 편집 후 seed 복원.

### Implementation for User Story 3

- [ ] T022 [US3] 안전장치 UI: `ui/agent-command-override-editor.tsx`에서 기본 프로필은 삭제 버튼 미제공·enable/disable 토글 제공, `canDisableProfile`이 false면 토글 비활성 + 사유 안내 문구 표시, backend 저장 거부 오류를 사용자 메시지로 표출
- [ ] T023 [P] [US3] Storybook: 마지막 활성 기본 프로필 disable 차단 상태, disabled 프로필 표시 상태 스토리 추가(`stories/organisms.stories.tsx`)
- [ ] T024 [US3] US3 검증: quickstart S3 수행(새 환경 seed, 차단 안내, 목록 제외, 파일 편집 복원) + 자동 테스트

**Checkpoint**: 세 user story 모두 독립 검증 완료.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 하위 호환·엣지 종단 검증과 문서 마무리.

- [ ] T025 하위 호환·엣지 종단 검증: quickstart S4(legacy-only 데이터 무조치 로드·동일 실행) + S5(PATH 결합 spawn 정상, 실행 중 세션에 disable 영향 없음, reuse 세션 동작) 수행
- [ ] T026 [P] 문서: `docs/`의 ACP command override 관련 문서(있는 경우)에 프로필/env 모델 반영 또는 신규 설정 가이드 추가(영문 파일명, 한국어 본문), 이슈 #121 수용 기준 대조표 포함
- [ ] T027 전체 자동 검증: `pnpm --filter agentic-workbench check-types && pnpm --filter agentic-workbench test` + `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`, 앱 간 import 미도입 확인
- [ ] T028 GitHub 이슈 #121의 수용 기준(1부 6항 + 확장 7항)을 체크리스트로 대조해 미충족 항목 없음을 확인하고 이슈에 결과 코멘트

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (타입)** → **Phase 2 (순수 로직)** → user stories
- **US1 (P1)**: Phase 2 완료 후 — 다른 story 의존 없음 (세션 선택 UI 변경 불필요: 기본 프로필 id = 기존 agent id)
- **US2 (P2)**: Phase 2 완료 후. T018은 T013(env-var-editor)을 재사용하므로 US1 이후 권장
- **US3 (P3)**: 판정 로직은 Phase 2에 있음. UI(T022)는 T018(프로필 편집기) 이후
- **Polish**: 전 story 완료 후

### Within Each User Story

- 테스트(T003/T004, T008/T009, T016)는 구현 전 작성·FAIL 확인
- model(순수 로직) → entities api → features UI → 검증 순
- 같은 파일 접점: `command-overrides.ts`(T005→T010), `agent-command-override-editor.tsx`(T013→T018→T022), `agent-run-panel.tsx`(T011의 run-panel-state와 T019) — 순차 진행

### Parallel Opportunities

```text
Phase 1: T001 ∥ T002
Phase 2: T003 ∥ T004 → T005 ∥ T006 → T007
US1:     T008 ∥ T009 → T010 → T011 ∥ T012 → T013 → T014 ∥ T015
US2:     T016 → T017 → T018 ∥ T019 → T020 ∥ T021
US3:     T022 → T023 ∥ T024
Polish:  T025 → T026 ∥ T027 → T028
```

---

## Parallel Example: Phase 2

```bash
# 테스트 먼저 (frontend/backend 병렬):
Task: "T003 command-overrides/profile-invariants 모델 테스트"
Task: "T004 agent_run_settings_service normalization/seed/불변식 테스트"

# 구현 (양쪽 병렬):
Task: "T005 frontend normalization·seed·invariants 구현"
Task: "T006 backend normalization·seed·저장 거부 구현"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1(타입) + Phase 2(순수 로직) 완료
2. Phase 3(US1) 완료 → quickstart S1로 독립 검증
3. **STOP and VALIDATE**: env 주입이 이슈 1부 수용 기준을 충족하면 이 시점에 배포/데모 가능

### Incremental Delivery

1. US1 → env 주입 (이슈 1부 완결, MVP)
2. US2 → 프로필 복수 등록·세션 선택 (이슈 확장 요청 핵심)
3. US3 → 안전장치 UI 노출 (로직은 Phase 2에서 이미 보장)
4. Polish → 하위 호환 종단 검증 + 이슈 수용 기준 대조

---

## Notes

- [P] = 다른 파일·의존 없음. [Story] = spec.md user story 추적용
- env value는 테스트·오류 메시지·로그 어디에도 노출 금지(secret 완화 방침)
- 각 task 또는 논리 그룹 완료 시 커밋, 각 checkpoint에서 독립 검증 가능

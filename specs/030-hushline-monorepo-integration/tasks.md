---
description: "Task list for Hushline 모노레포 편입 및 Agent Run 기능 추가"
---

# Tasks: Hushline 모노레포 편입 및 Agent Run 기능 추가

**Input**: Design documents from `/specs/030-hushline-monorepo-integration/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: 사용자-여정 테스트는 선택. 단, Constitution이 요구하는 테스트(공유 crate 순수 로직,
직렬화 계약, 경로/소유 안전 경계)는 **필수**이며 `[Required test]`로 표기한다.

**Organization**: 작업은 user story별로 그룹화되어 각 스토리를 독립 구현·검증할 수 있다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 가능(서로 다른 파일, 미완료 의존 없음)
- **[Story]**: US1/US2/US3 (spec.md 매핑)
- 각 설명에 정확한 파일 경로 포함

## Path Conventions

- 편입 앱 프론트: `apps/hushline/src/{app,pages,features,entities,shared,widgets,components/ui}`
- 편입 앱 백엔드: `apps/hushline/src-tauri/src/{domain,application,adapters,ports.rs}`
- 공유 Rust 코어: `crates/acp-agent-core/src/{domain,application,ports,infrastructure}`
- 공유 TS 클라이언트: `packages/agent-client/src`
- 문서: `docs/*.md`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 편입/추출 작업을 위한 저장소 준비

- [X] T001 브랜치 `030-hushline-monorepo-integration`에 설계·스펙 문서 커밋 (`docs/20260721-acp-agent-core-reuse-strategy.md`, `specs/030-hushline-monorepo-integration/*`)
- [X] T002 병합 전 레이아웃 대조: `pnpm-workspace.yaml`, 루트 `Cargo.toml`, `turbo.json`을 hushline 레포와 비교해 편입 지점 확정 (참조: `docs/20260721-acp-agent-core-reuse-strategy.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 스토리가 의존하는 편입 배선(그룹 A)과 agent 스토리가 의존하는 공유 코어 추출(그룹 B)

**⚠️ CRITICAL**: 이 단계가 끝나기 전에는 어떤 user story 작업도 시작할 수 없다.

### Group A — Hushline 모노레포 편입 (모든 스토리 선행)

- [X] T003 hushline을 `apps/hushline`로 스냅샷 복사(방식 C, 빌드 산출물 제외). 앱 패키지명 `@yoophi/hushline` 유지
- [X] T004 루트 `Cargo.toml` workspace `members`에 `apps/hushline/src-tauri` 추가
- [X] T005 `turbo.json`은 dest 상위집합이라 편집 불필요(hushline 태스크 전부 포함); hushline tsconfig는 자체 완결형이라 base 상속 조정 불필요
- [X] T006 `pnpm install`로 워크스페이스 인식 확인 (`@yoophi/hushline` 인식됨)

### Group B — 공유 코어 추출 (US2/US3 선행, workbench 무동작 리팩터)

- [X] T007 `crates/acp-agent-core` 생성(edition 2021) — 루트 `Cargo.toml`의 `crates/*` glob이 자동 등록
- [X] T008 [P] domain 모듈 이동 (run, events, agent, acp_session, agent_tool_candidate) — `domain/permission.rs`는 존재하지 않아 제외(권한 타입은 `ports/permission`)
- [X] T009 [P] ports 모듈 이동 (session_launcher, session_handle, session_registry, event_sink, acp_session_store, agent_catalog, permission)
- [X] T010 application 유스케이스 이동 (start_agent_run, send_prompt, cancel_agent_run, set_permission_mode, cancel_prompt_and_send, steer_prompt, agent_run_errors)
- [X] T011 infrastructure 이동 (acp/* 전체, agent_session_registry, permission_broker, agent_catalog, noop_acp_session_store)
- [X] T012 `AcpSessionStore` 포트를 코어에 유지, `noop_acp_session_store`를 코어로 이동. `json_acp_session_store`(Tauri)는 workbench 잔류하며 코어 포트 구현
- [X] T013 env `ACP_MAX_RUNS`(신규 우선) + `ACP_WORKBENCH_MAX_RUNS`(레거시 폴백) 둘 다 인식
- [X] T014 `crates/acp-agent-core/src/lib.rs`에서 domain/ports/application/infrastructure 4개 모듈 공개(전체 경로 소비). 편의 top-level re-export는 폴리시 단계 여지
- [X] T015 [Required test] RunEvent camelCase 직렬화 테스트 (events.rs와 함께 이동, 84개 통과에 포함)
- [X] T016 [Required test] registry 소유자/상한 테스트 (registry와 함께 이동, 84개 통과에 포함)
- [ ] T017 `packages/agent-client` 생성 및 workbench `entities/agent-run`의 계약 하위집합 types/repository 이동·re-export
- [ ] T018 [Required test] TS↔Rust 계약 스냅샷 테스트 (RunEventEnvelope/AgentRunRequest camelCase) in `packages/agent-client/src`
- [X] T019 `agentic-workbench`(src-tauri)가 `acp-agent-core`를 소비하도록 mod.rs re-export로 교체. (`@yoophi/agent-client` 프론트 소비는 T017 이후)
- [~] T020 [Atomic verify] Rust 완료: `cargo test -p acp-agent-core`(84 pass), `cargo test -p agentic-workbench`(113 pass), `cargo check --workspace`(green), `pnpm check-types`(10/10). TS 패키지 검증은 T017/T018 이후

**Checkpoint**: 편입 + 공유 코어 준비 완료 — user story 구현 시작 가능

---

## Phase 3: User Story 1 - Hushline을 모노레포에서 그대로 실행 (Priority: P1) 🎯 MVP

**Goal**: 편입된 hushline의 기존 자막 생성·queue 관리가 회귀 없이 동작하고 공용 파이프라인에 포함됨.

**Independent Test**: 모노레포에서 hushline 실행 → 자막 생성·queue 추가/삭제·진행 표시가 편입 전과 동일; 공용 build/check-types/test 통과 (quickstart V1).

- [X] T021 [US1] `pnpm build` + `pnpm check-types`가 hushline 포함해 통과함을 확인·수정 (모노레포 hoist된 `@types/react@19`를 lucide-react가 폴백 해석하던 이슈를 `apps/hushline/tsconfig.json`의 `paths`로 로컬 18 고정하여 해결; hushline·전체 워크스페이스 check-types/build 그린)
- [X] T022 [US1] `cargo test -p hushline` 통과 확인·수정 (exit 0, 2 passed: domain URL 검증·출력명 sanitize)
- [~] T023 [US1] 수동 검증 (quickstart V1). 자동 검증 완료: 앱 실행(PID)·Vite UI HTTP 200·command 등록(check_dependencies/process_video 등) 정상 → 편입 무회귀 입증. **미완(환경 의존)**: 실제 자막 생성/queue 클릭-스루는 `yt-dlp`+Whisper 엔진 미설치로 이 환경에서 불가(편입과 무관). 도구 설치 후 실행 창에서 사용자 확인 필요.

**Checkpoint**: US1 독립 완료 — 편입 무회귀 확보(SC-001)

---

## Phase 4: User Story 2 - 자막을 원하는 방식으로 정리해 새 문서로 저장 (Priority: P2)

**Goal**: 자막에 대해 정리 방식을 지정해 agent run 실행 → 스트리밍 표시 → 새 문서 저장.

**Independent Test**: 결과 카드에서 "정리하기" 실행 → 스트리밍 표시 → 저장 후 재열람 (quickstart V2·V3·V4).

### 백엔드 (apps/hushline/src-tauri)

- [ ] T024 [US2] `Cargo.toml`에 `acp-agent-core` 의존 추가 및 `AppState` manage 등록 in `apps/hushline/src-tauri/src/adapters/tauri.rs`
- [ ] T025 [US2] `HushlineAgentSink`(RunEventSink 구현) 작성 — `agent-run-event` 채널 emit in `apps/hushline/src-tauri/src/adapters/tauri.rs`
- [ ] T026 [US2] `start_agent_run` command 등록 + 입력 검증(`goal` 비어있지 않음, `cwd` 관리 폴더 하위) + `generate_handler!` 반영 in `apps/hushline/src-tauri/src/adapters/tauri.rs`
- [ ] T027 [P] [US2] [Required test] cwd 경로 검증 거부 테스트 (계약 CT-2) in `apps/hushline/src-tauri/src`
- [ ] T028 [US2] `OrganizedDocument` 도메인 + 저장 포트/어댑터 (경로·크기·UTF-8 검증) in `apps/hushline/src-tauri/src/{domain,application,adapters}`
- [ ] T029 [US2] 단일 기본 agent 실행 카탈로그/명령 최소 설정 제공 in `apps/hushline/src-tauri/src/adapters`

### 프론트 (apps/hushline/src)

- [ ] T030 [P] [US2] `package.json`에 `@yoophi/agent-client` 의존(`workspace:*`) 추가 in `apps/hushline/package.json`
- [ ] T031 [US2] `shared/api`에 agent-client 재노출 (startAgentRun, cancelAgentRun, listenRunEvents) in `apps/hushline/src/shared/api`
- [ ] T032 [P] [US2] `entities/organized-document` 모델·API 어댑터 추가 in `apps/hushline/src/entities/organized-document`
- [ ] T033 [US2] `features/organize-transcript`: `useAgentRun` 훅(startAgentRun + listenRunEvents로 AgentMessage 누적) in `apps/hushline/src/features/organize-transcript`
- [ ] T034 [US2] result-grid 카드에 "정리하기" 액션 + 정리 방식 입력 UI in `apps/hushline/src/widgets/result-grid`, `apps/hushline/src/features/organize-transcript`
- [ ] T035 [US2] AgentMessage 스트림 누적 → OrganizedDocument 저장 및 목록 재열람 UI in `apps/hushline/src/{features/organize-transcript,widgets/result-grid}`
- [ ] T036 [US2] 취소/오류 처리 UI (cancelAgentRun 연결, `Error`·`Lifecycle(Cancelled)` 이벤트 표시) in `apps/hushline/src/features/organize-transcript`
- [ ] T037 [US2] 검증: quickstart V2(스트리밍)·V3(저장·재열람, SC-002)·V4(취소·오류, SC-005·SC-006)

**Checkpoint**: US2 독립 완료 — 정리→저장 end-to-end (agent run MVP)

---

## Phase 5: User Story 3 - 정리 문서 기반 지식 대화 (Priority: P3)

**Goal**: 저장된 문서를 대상으로 세션 내 다회 대화(맥락 유지).

**Independent Test**: 저장 문서에서 대화 시작 → 후속 질문 2회+ → 이전 맥락 반영 답변; 종료 시 run 정리 (quickstart V5).

### 백엔드 (apps/hushline/src-tauri)

- [ ] T038 [US3] `send_prompt_to_run`, `respond_agent_permission`, `set_run_permission_mode` command 등록 + `generate_handler!` 반영 in `apps/hushline/src-tauri/src/adapters/tauri.rs`
- [ ] T039 [P] [US3] [Required test] 미소유 `runId`에 대한 프롬프트/취소 거부 테스트 (계약 CT-3) in `apps/hushline/src-tauri/src`
- [ ] T040 [US3] `ChatSession` 도메인 + 저장(메시지 로그) in `apps/hushline/src-tauri/src/{domain,application,adapters}`
- [ ] T043 [US3] 창/앱 종료 시 소유 run 정리 연결 (계약 CT-4 hushline 측) in `apps/hushline/src-tauri/src/adapters/tauri.rs`

### 프론트 (apps/hushline/src)

- [ ] T041 [US3] `features/chat-with-document`: 세션 시작 + `sendPromptToRun` 반복 + 스트림 렌더 in `apps/hushline/src/features/chat-with-document`
- [ ] T042 [US3] 문서 뷰/result-grid에 "대화하기" 액션 + 대화 UI(메시지 목록·입력) in `apps/hushline/src/{widgets,features/chat-with-document}`
- [ ] T044 [US3] 검증: quickstart V5(다회 대화 맥락 유지 SC-004, 종료 시 자원 정리 SC-005)

**Checkpoint**: US3 독립 완료 — 세션 내 다회 대화

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T045 [P] `docs/20260721-acp-agent-core-reuse-strategy.md`에 실제 편입/추출 결과 반영 갱신
- [ ] T046 [P] 신규 재사용 UI가 있으면 Storybook atomic 분류(atoms/molecules/organisms/pages) 등록 in `apps/hushline/src/stories`
- [ ] T047 [P] 전체 계약/픽스처 테스트 재실행 및 atomic cross-app 검증 결과 문서화 (`cargo test -p acp-agent-core`, `-p agentic-workbench`, `-p hushline`, `pnpm check-types`)
- [ ] T048 [P] 루트 `README.md` Apps 표에 hushline 추가 및 프로젝트 구조 반영

---

## Dependencies & Execution Order

- **Phase 1 (Setup)** → **Phase 2 (Foundational)** → 이후 스토리.
- **Group A(T003–T006)** 는 모든 스토리 선행. **Group B(T007–T020)** 는 US2·US3 선행(US1은 Group B 불필요하나 순서상 Foundational 이후 검증).
- **US1(P1)**: Group A 완료 후 검증 가능(MVP).
- **US2(P2)**: Group A + Group B 완료 필요.
- **US3(P3)**: US2의 command 배선·저장 모델 위에 확장(T038은 T026 이후, ChatSession은 OrganizedDocument 패턴 재사용).
- **Polish**: 모든 스토리 이후.

## Parallel Opportunities

- Foundational: `T008`·`T009`(domain/ports 이동)은 병렬. `T015`·`T016`·`T018`(테스트)은 각 대상 완료 후 병렬.
- US2: `T027`(백엔드 테스트)·`T030`(프론트 의존)·`T032`(entities)는 서로 다른 파일이라 병렬 가능.
- US3: `T039`(테스트) 병렬.
- Polish: `T045`–`T048` 전부 병렬.

## Implementation Strategy (MVP first)

1. **MVP = Phase 1 + Phase 2 + US1**: hushline이 모노레포에서 회귀 없이 동작 + 공유 코어 추출 완료.
2. **첫 agent 가치 = US2**: 정리→스트리밍→저장. walking skeleton(T024–T026 + T033 + 버튼 1개, quickstart V2)을 먼저 통과시킨 뒤 저장/취소로 확장.
3. **확장 = US3**: 다회 대화.
4. 각 스토리 완료 시 quickstart 해당 V 시나리오로 독립 검증.

## Notes

- **범위 밖(후속 스펙)**: MCP 서빙/오케스트레이션(설계 Phase 4~5), ACP 세션 재개 영속, TS 타입 자동생성(ts-rs/specta).
- Constitution 원칙 V에 따라 `crates/*`·`packages/*` 변경(Phase 2)은 소비 앱(workbench, hushline) 검증까지 완료해야 done.

---

description: "독립 Workbench 서버의 모든 자식 프로세스를 공통 감독 경계로 옮기는 실행 작업"
---

# Tasks: 서버 자식 프로세스 감독

**Input**: `/specs/045-process-supervisor/`의 spec, plan, research, data model, contracts

**Tests**: lifecycle, CAS/outbox, process tree, output, inventory는 안전 경계이므로 테스트와 격리 fixture를 구현보다 먼저 작성한다. filtered 0은 통과 근거가 아니다.

**Organization**: platform feasibility는 모든 production consumer migration의 선행 gate다. read-only helper의 domain 의미는 transient로 유지하고, containment recovery anchor만 durable하게 관리한다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 서로 다른 파일에서 선행 작업 없이 병렬 실행 가능
- **[Story]**: spec의 사용자 스토리
- 모든 task는 실제 변경 또는 검증 파일 경로를 포함한다.

## Phase 1: Setup — 범위와 격리 spike 기반

**Purpose**: production 변경 없이 공통 crate와 정확한 inventory, platform spike harness를 만든다.

- [ ] T001 `crates/process-supervisor/Cargo.toml`, `crates/process-supervisor/src/lib.rs`, root `Cargo.toml`에 재사용 supervisor crate skeleton과 target별 dependency boundary를 추가한다
- [ ] T002 `scripts/check-process-spawn-inventory.py`와 `crates/process-supervisor/tests/process_inventory.rs`에 exact-path production spawn inventory gate를 작성하고 `specs/045-process-supervisor/contracts/process-inventory.md`의 모든 범주를 대조한다
- [ ] T003 [P] `crates/process-supervisor/tests/fixtures/process_tree.rs`에 direct/grandchild, leader-exit, new group/session, double-fork+reparent, control-FD-close, env-clear+exec, signal-ignore fixture mode를 만든다
- [ ] T004 [P] `crates/process-supervisor/tests/fixtures/protocol_peer.rs`에 exact-limit, limit+1, malformed, mid-frame EOF, slow-loris, endless-valid-frame fixture mode를 만든다

---

## Phase 2: Foundational — Feasibility 및 공통 안전 경계

**Purpose**: 실제 플랫폼 API/권한과 핵심 상태 전이를 입증한다. 이 phase가 끝나기 전 production consumer를 변경하지 않는다.

**⚠️ CRITICAL GATE**: macOS/Linux/Windows target 중 하나라도 required containment를 입증하지 못하면 실패 근거와 대안을 설계 리뷰로 되돌린다. group kill, nonce 상속만의 통과, skip, threshold 완화로 다음 phase를 열지 않는다.

- [ ] T005 `crates/process-supervisor/tests/platform_feasibility.rs`에 macOS/Linux의 process inventory 권한, env-clear+exec escape, keeper-only/server+keeper hard kill, safe identity-check-and-signal race를 재현하는 non-zero isolated tests를 먼저 작성한다
- [ ] T006 [P] `crates/process-supervisor/tests/windows_job_feasibility.rs`에 suspended create, Job assign-before-resume, breakaway denial, kill-on-close와 direct wait fixture를 먼저 작성한다
- [ ] T007 `crates/process-supervisor/src/platform/unix/feasibility.rs`에서 macOS/Linux 실제 public API와 배포 권한으로 reusable identity handle·descendant acquisition·signal 원자성을 spike하고 T005의 각 case를 PASS 또는 구체적 blocker로 판정한다
- [ ] T008 [P] `crates/process-supervisor/src/platform/windows/feasibility.rs`에서 Windows Job Object spike를 구현하고 T006을 실제 Windows target에서 실행 가능하게 한다
- [ ] T009 `.github/workflows/quality.yml`에 macOS Apple Silicon, Linux x86_64, Windows x86_64 feasibility jobs를 추가하고 `specs/045-process-supervisor/platform-evidence.md`에 API, 권한, test 수, exit code, leak/identity 결과를 기록한다
- [ ] T010 `specs/045-process-supervisor/platform-evidence.md`의 prerequisite matrix를 판정해 모든 required target 증거가 있을 때만 T011 이후를 시작하고, 실패 target은 `specs/045-process-supervisor/review-ledger.md`에 새 설계 review input으로 기록한다
- [ ] T011 `crates/process-supervisor/src/spec.rs`와 `crates/process-supervisor/src/state.rs`에 `ProcessSpec`, owner/attempt, stream policy, `Reserved/Spawning/Adopted/Published/Active/Aborting/Terminal` reducer를 구현한다
- [ ] T012 [P] `crates/workbench-core/src/ports/process_attempt_store.rs`와 `crates/workbench-core/src/ports/mod.rs`에 domain publication과 containment recovery anchor를 분리한 store port 및 typed CAS result를 정의한다
- [ ] T013 `crates/workbench-core/src/infrastructure/sqlite_ledger.rs`에 schema v2→v3 `process_attempt`/`process_publication_outbox` migration, conditional transition, unique event id, retention GC를 구현한다
- [ ] T014 `crates/workbench-core/tests/process_publication.rs`에 publication-wins/cleanup-wins barrier, ambiguous commit, storage fault, commit/send/ack crash, reconnect replay와 projection dedupe 회귀시험을 먼저 작성해 T013/T015의 실패를 확인한다
- [ ] T015 `crates/workbench-core/src/application/process_publication.rs`에 domain result + `Adopted→Published` + outbox atomic transaction과 `Adopted→Active/Aborting` resolver를 구현한다
- [ ] T016 `crates/process-supervisor/src/registry.rs`에 cancellation-safe child ownership, `UnpublishedProcessLease`, no-ack resolver, quarantined readiness, terminal outcome arbitration을 구현한다

**Checkpoint**: feasibility evidence와 CAS/outbox/lease foundation이 모두 실제 non-zero tests로 통과했다. 이제 production consumer migration을 시작할 수 있다.

---

## Phase 3: User Story 1 — 시작했다고 거짓 보고하지 않기 (Priority: P1) 🎯 MVP

**Goal**: spawn/adopt/publication이 끝난 뒤에만 accepted/started를 한 durable logical event로 공개한다.

**Independent Test**: spawn/adopt 실패, future 취소, CAS 양쪽 winner, commit/send crash를 반복해 실패 시 started/live child 0, 성공 시 attempt별 logical publication과 projection 적용 1회를 확인한다.

### Tests for User Story 1

- [ ] T017 [P] [US1] `crates/acp-agent-core/src/application/start_agent_run.rs` tests에 HTTP/command response가 adoption/publication 뒤이면서 전체 ACP turn 완료 전 반환되는 barrier 시험을 추가한다
- [ ] T018 [P] [US1] `crates/acp-agent-core/src/infrastructure/acp/runner.rs` tests에 missing executable/adopt failure/cancel 각 100회에서 Started/live child 0, 정상 시작 100회에서 attempt별 Started logical 1회를 검증한다
- [ ] T019 [P] [US1] `crates/acp-agent-core/src/infrastructure/acp/terminal.rs` tests에 terminal create publication CAS, reply loss/retry, old-attempt late completion 격리를 추가한다
- [ ] T020 [P] [US1] `crates/workbench-core/tests/process_publication.rs`에 outbox WS replay와 same event id client projection dedupe를 연결해 reconnect 뒤 사용자 관측 적용 1회를 검증한다

### Implementation for User Story 1

- [ ] T021 [US1] `crates/process-supervisor/src/lib.rs`와 `crates/process-supervisor/src/registry.rs`에 prepare/adopt와 drive-to-completion을 분리한 async process port를 공개한다
- [ ] T022 [US1] `crates/acp-agent-core/src/application/start_agent_run.rs`에서 public start를 adoption/publication까지만 기다리고 장시간 ACP turn은 background로 유지한다
- [ ] T023 [US1] `crates/acp-agent-core/src/infrastructure/acp/runner.rs`를 supervisor protocol process로 이관하고 current Started-before-spawn/adopt 순서를 제거한다
- [ ] T024 [US1] `crates/acp-agent-core/src/infrastructure/acp/terminal.rs`를 durable terminal owner/outbox와 supervisor display process로 이관한다
- [ ] T025 [US1] `crates/workbench-core/src/infrastructure/run/acp_run_engine.rs`에 publication store와 process supervisor를 주입하고 retry가 같은 attempt/result/event를 반환하게 한다
- [ ] T026 [US1] `crates/workbench-host/src/assembly.rs`와 `crates/workbench-host/src/http.rs`에서 server-wide supervisor/store/outbox를 조립하고 민감 값을 제외한 owner/purpose/lifecycle/exit/current count diagnostics를 status에 연결한다

**Checkpoint**: User Story 1은 agent run/terminal 시작 진실성과 비동기 장시간 turn 계약을 독립적으로 검증할 수 있다.

---

## Phase 4: User Story 2 — 취소와 서버 종료가 자식 트리를 남기지 않기 (Priority: P1)

**Goal**: cancel, timeout, shutdown, keeper death, server crash에서 descendant와 unreaped direct child를 0으로 만든다.

**Independent Test**: 각 지원 target에서 escape fixture에 graceful/force, keeper-only kill, server+keeper kill을 적용하고 unrelated PID-reuse 대조 process 생존을 확인한다.

### Tests for User Story 2

- [ ] T027 [P] [US2] `crates/process-supervisor/tests/unix_containment.rs`에 group 생존 수와 identity live set을 분리 기록하는 leader/session/double-fork/env-clear/keeper-death 회귀시험을 추가한다
- [ ] T028 [P] [US2] `crates/process-supervisor/tests/windows_containment.rs`에 Job active count, breakaway denial, graceful/force, server-crash kill-on-close, direct wait 회귀시험을 추가한다
- [ ] T029 [P] [US2] `crates/workbench-host/tests/process_supervisor_shutdown.rs`에 live keeper death takeover, server drain/force barrier, startup unfinished-anchor reconcile와 desktop daemon launcher의 독립 daemon/readiness/stop 불변식 시험을 추가한다

### Implementation for User Story 2

- [ ] T030 [US2] `crates/process-supervisor/src/platform/unix.rs`에 prerequisite에서 입증된 containment/identity handle, keeper control, graceful/force, quiescence, reap를 구현한다
- [ ] T031 [US2] `crates/process-supervisor/src/platform/windows.rs`에 suspended spawn, Job assign-before-resume, kill-on-close, graceful/force와 wait를 구현한다
- [ ] T032 [US2] `crates/workbench-host/src/process_keeper.rs`와 `apps/agentic-workbench-server/src/main.rs`에 internal keeper mode와 handshake를 조립한다
- [ ] T033 [US2] `apps/agentic-workbench/src-tauri/src/main.rs`에 embedded compatibility keeper entry를 조립하되 daemon bootstrap ownership은 변경하지 않는다
- [ ] T034 [US2] `crates/process-supervisor/src/registry.rs`에 keeper handle 감시, live-server cleanup takeover, combined-crash anchor recovery와 idempotent terminal arbitration을 구현한다
- [ ] T035 [US2] `crates/workbench-host/src/lifecycle/server.rs`에 startup reconcile-before-readiness와 `shutdown_all(deadline)` live=0/unreaped=0 barrier를 연결한다

**Checkpoint**: User Story 2는 각 target의 실제 containment matrix로 독립 검증되며 다른 target 결과로 대체하지 않는다.

---

## Phase 5: User Story 3 — 과도한 출력에도 서버가 응답하기 (Priority: P2)

**Goal**: protocol 정확성을 보존하면서 stdout/stderr pressure에서도 memory와 control latency를 제한한다.

**Independent Test**: protocol boundary/slow-loris/endless frames와 display 100 MiB/no-newline를 실행해 typed outcome, 후속 frame 0, bounded counters, 2초 이내 status/cancel을 확인한다.

### Tests for User Story 3

- [ ] T036 [P] [US3] `crates/process-supervisor/tests/output_policy.rs`에 protocol exact/+1/malformed/EOF/progress-timeout, parsed-capture overflow, display pressure와 secret sentinel 시험을 추가한다
- [ ] T037 [P] [US3] `crates/acp-agent-core/src/infrastructure/acp/transport.rs` tests에 protocol fatal outcome 뒤 pending request 실패와 다음 frame 미전달을 검증한다

### Implementation for User Story 3

- [ ] T038 [US3] `crates/process-supervisor/src/output.rs`에 `ProtocolFrames`, `ParsedCapture`, `DisplayLog`, `Null` drain을 별도 bounded policy로 구현한다
- [ ] T039 [US3] `crates/process-supervisor/src/output.rs`에 incomplete-frame progress deadline/minimum progress와 owner runtime deadline arbitration을 구현한다
- [ ] T040 [US3] `crates/acp-agent-core/src/infrastructure/acp/transport.rs`에서 ACP stdout을 exact protocol policy, stderr를 display policy에 연결한다
- [ ] T041 [US3] `crates/process-supervisor/src/registry.rs`에 drain/wait/cancel 동시성 및 typed output failure의 tree termination 연결을 구현한다

**Checkpoint**: User Story 3은 protocol 무손상과 bounded display/capture를 각각 독립적으로 검증한다.

---

## Phase 6: User Story 4 — 모든 서버 소유 실행을 같은 정책으로 다루기 (Priority: P2)

**Goal**: ACP 외 Git/watcher/catalog/PATH helper까지 공통 supervisor를 사용하고 inventory drift를 막는다.

**Independent Test**: production source scan에서 ServerOwned direct spawn 0, 미분류 0을 확인하고 각 helper의 정상/timeout/cancel/overflow/fallback 사용자 결과를 회귀 검증한다.

### Tests for User Story 4

- [ ] T042 [P] [US4] `crates/process-supervisor/tests/blocking_capture.rs`에 sync helper의 timeout/cancel/shutdown, complete capture, overflow, Tokio worker 비점유 시험을 추가한다
- [ ] T043 [P] [US4] `crates/git-core/src/git_cli.rs` tests에 history/detail/diff/status/worktree command의 injected supervised capture 회귀시험을 추가한다
- [ ] T044 [P] [US4] `crates/acp-agent-core/src/infrastructure/agent_catalog.rs`와 `crates/acp-agent-core/src/infrastructure/acp/util.rs` tests에 curl/PATH timeout, fallback, output bound 회귀시험을 추가한다

### Implementation for User Story 4

- [ ] T045 [US4] `crates/process-supervisor/src/blocking.rs`에 registry/platform launcher를 공유하는 bounded blocking capture facade와 async adapter를 구현한다
- [ ] T046 [US4] `crates/git-core/src/git_cli.rs`와 `crates/workbench-core/src/infrastructure/git/cli_branch_provider.rs`, `cli_remote_provider.rs`, `cli_worktree_provider.rs`, `cli_worktree_change_provider.rs`를 injected supervised capture로 이관한다
- [ ] T047 [US4] `crates/workbench-core/src/infrastructure/fs/worktree_watcher.rs`와 `crates/workbench-core/src/infrastructure/orchestration/worktree_guard.rs`의 Git probe를 transient-domain/durable-anchor supervised capture로 이관한다
- [ ] T048 [US4] `crates/acp-agent-core/src/infrastructure/agent_catalog.rs`의 curl과 `crates/acp-agent-core/src/infrastructure/acp/util.rs`의 login-shell PATH probe를 supervised capture로 이관한다
- [ ] T049 [US4] `crates/workbench-protocol/src/operations/agent.rs`, `crates/workbench-protocol/src/operations/run.rs`, `crates/workbench-protocol/src/fault.rs`에 cwd/executable/filesystem path가 server host 기준임을 공개 계약과 validation error에 명시하고 remote contract tests를 추가한다
- [ ] T050 [US4] `scripts/check-process-spawn-inventory.py`를 workspace validation에 연결해 platform module과 exact daemon/native/build/fixture/other-app path 외 direct spawn을 실패시킨다

**Checkpoint**: User Story 4의 production ServerOwned inventory가 100% 공통 경계를 통과한다.

---

## Phase 7: Polish & Cross-Cutting Verification

**Purpose**: 문서, 실제 target evidence, consumer 회귀와 전체 gate를 최종 코드에 연결한다.

- [ ] T051 [P] `docs/workbench-process-supervision.md`에 owner/attempt, CAS/outbox, transient domain/recovery anchor, output, platform 운영 계약을 한국어와 Mermaid로 문서화한다
- [ ] T052 [P] `specs/045-process-supervisor/platform-evidence.md`에 최종 macOS/Linux/Windows test 수, exit code, API/권한, leak/reap/PID 대조 결과를 갱신한다
- [ ] T053 `specs/045-process-supervisor/review-ledger.md`에 구현 후 OCR delegate findings/verdict/coverage와 반영을 기록한다
- [ ] T054 `specs/045-process-supervisor/review-ledger.md`에 OCR 반영 후 최종 HEAD Codex adversarial `--wait` job id, 11+code 전체 범위, findings/verdict/반영을 기록한다
- [ ] T055 `specs/045-process-supervisor/quickstart.md`의 inventory, lifecycle, output, containment, consumer regression과 credential sentinel 전 산출물 0회를 filtered 0 없이 실행하고 단계별 test 수와 exit code를 `specs/045-process-supervisor/validation.md`에 기록한다
- [ ] T056 root `package.json`의 8단계 전체 gate를 최종 HEAD에서 실행하고 실패 명령도 exit code를 보존해 `specs/045-process-supervisor/validation.md`에 기록한다
- [ ] T057 `specs/045-process-supervisor/checklists/requirements.md`를 실제 prerequisite/target/전체 gate 증거에 맞게 갱신하고 증거 없는 항목은 완료 표시하지 않는다

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 즉시 시작 가능
- **Foundational (Phase 2)**: Setup 뒤 실행. T005/T006 red fixture → T007/T008 spike → T009 actual target jobs → T010 evidence 판정 순서다
- **Hard gate**: T010 prerequisite 판정 전에는 T011 이후와 production consumer 파일을 변경하지 않는다
- **User Story 1 (Phase 3)**: foundation 완료 뒤 시작
- **User Story 2 (Phase 4)**: foundation 완료 뒤 시작하며 US1의 registry/publication core를 재사용한다
- **User Story 3 (Phase 5)**: foundation 완료 뒤 시작하고 ACP 연결 T040은 T023 뒤다
- **User Story 4 (Phase 6)**: foundation과 blocking facade 뒤 consumer별로 진행한다
- **Polish (Phase 7)**: 모든 story 완료 뒤 OCR → Codex 순서와 전체 gate를 실행한다

### User Story Dependencies

- **US1**: T011–T016 foundation에 의존한다
- **US2**: T007–T010 platform evidence와 T016 registry에 의존한다
- **US3**: T011 spec/state에 의존하며 ACP transport 연결은 US1 runner migration 뒤다
- **US4**: T011/T016과 T045 blocking facade에 의존하며 helper별 migration은 서로 독립적이다

### Parallel Opportunities

- T003/T004 fixture 작성은 병렬 가능하다
- T005 Unix와 T006 Windows red tests, T007 Unix와 T008 Windows spike는 target별 병렬 가능하다
- US1의 T017–T020, US2의 T027–T029, US3의 T036–T037, US4의 T042–T044는 서로 다른 test file에서 병렬 가능하다
- T046–T048 consumer migration은 common facade가 고정된 뒤 파일군별 병렬 가능하다
- 실제 리뷰는 순차 제약 때문에 OCR 완료·반영 뒤 Codex를 시작한다

---

## Implementation Strategy

1. Phase 1에서 codebase 범위와 재현 fixture만 만든다.
2. Phase 2의 실제 platform spike로 요구 충족 가능성을 먼저 판정한다.
3. prerequisite가 통과한 target에서 CAS/outbox/lease foundation을 완성한다.
4. US1을 MVP로 구현해 truthful start와 비동기 ACP turn을 검증한다.
5. US2 tree containment, US3 output, US4 inventory migration을 순서대로 통합한다.
6. 구현 후 OCR delegate → 지적 반영 → Codex adversarial `--wait` → 지적 반영 → 전체 gate 순서를 지킨다.

## Done When

- [ ] T001–T057이 실제 증거에 맞게 완료됐다
- [ ] 세 target feasibility/containment matrix에 빈칸과 대체 근거가 없다
- [ ] production ServerOwned direct spawn과 미분류 경로가 0개다
- [ ] publication/cleanup CAS와 outbox crash/replay fixture가 모두 non-zero로 통과한다
- [ ] 구현 후 OCR/Codex 순차 리뷰와 최종 8단계 gate가 같은 final HEAD를 검증했다

# Implementation Plan: 서버 자식 프로세스 감독 (045)

**Branch**: `045-process-supervisor` | **Date**: 2026-09-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/045-process-supervisor/spec.md`

## Summary

044가 분리한 standalone server 안에서 직접 생성하는 ACP agent, terminal, Git, watcher helper, catalog `curl`, login-shell PATH probe를 새 공통 crate `process-supervisor`로 모은다. durable business execution은 domain owner를 먼저 예약하고, read-only helper의 domain 의미는 transient로 유지한다. 모든 ServerOwned child는 별도의 durable containment recovery anchor를 갖는다. supervisor가 child tree containment와 PID/start identity를 소유한 뒤 `UnpublishedProcessLease`를 돌려주고 publication CAS와 outbox handoff가 끝나야 accepted/started를 공개한다. 현재 `acp/runner.rs`의 `Started → Command::spawn()` 순서는 `reserve → spawn → adopt → publish → accepted/started`로 바뀐다.

Unix는 같은 실행 파일의 내부 keeper mode와 attempt identity ledger를 함께 사용한다. process group은 빠른 정상 경로이고, nonce+start identity inventory는 leader 조기 종료·새 session/group·double-fork로 이탈한 live descendant를 찾는 완료 판정이다. parent control pipe EOF 뒤에도 keeper가 group과 identity 집합을 모두 종료·검증한다. 이 추적을 실제 target escape fixture로 증명하지 못하면 해당 target은 supervisor capability를 제공하지 않고 feature 완료를 막는다. Windows는 suspended process를 breakaway 없는 Job Object에 먼저 넣고 resume해 post-spawn assignment race를 없앤다. display log, parsed capture, ACP JSON-RPC protocol stream을 서로 다른 출력 정책으로 다룬다. protocol frame은 임의 truncate/drop하지 않으며 최대 frame 위반·malformed·중간 EOF를 typed failure로 끝내고 process tree를 정리한다.

## Technical Context

**Language/Version**: Rust 1.98(workspace). TypeScript/UI 변경 없음.

**Primary Dependencies**:
- 기존: `tokio`, `libc`, `serde`, `uuid`, `thiserror`/`anyhow`(consumer boundary).
- Unix: stable `CommandExt::process_group`, `libc` signal/pipe/FD primitives, target process inventory API. 같은 composition-root executable의 내부 keeper entrypoint와 attempt nonce/start-identity ledger.
- Windows: target-specific `windows-sys` Process Threading/Job Object/Pipes APIs. `CREATE_SUSPENDED → AssignProcessToJobObject → ResumeThread`.
- 새 third-party process wrapper는 도입하지 않는다. lifecycle ordering과 cancellation semantics를 AW 계약에 맞게 직접 고정한다.

**Storage**: 기존 `workbench-ledger.sqlite3`의 additive v3는 platform-neutral `process_publication`과 `process_publication_outbox`만 추가한다. 현재 `SessionRegistry::reserve_run`과 terminal registry는 memory-only이므로 durable store라고 간주하지 않는다. T010 뒤 T016은 별도 additive migration으로 `process_attempt` containment anchor와 retention을 추가하고 v3 publication winner와 원자적으로 연결한다. long-lived ACP run/terminal과 side-effecting Git/worktree mutation은 domain intent/result를 기존 ledger identity에 연결한다. read-only Git, watcher probe, catalog curl, login-shell probe는 domain result/retry 의미는 transient지만, 모든 ServerOwned child는 keeper/server combined crash recovery를 위한 containment anchor를 spawn 전에 만든다. terminal 뒤 anchor는 bounded retention 후 GC하며 env/credential은 저장하지 않는다.

**Testing**:
- prerequisite feasibility spike: target별 safe identity handle, env-clear escape, keeper-only/server+keeper hard kill, 실제 권한.
- `process-supervisor` 단위/통합 fixture: publication/abort CAS 양쪽 winner, outbox crash/replay, spawn/adopt failure, future cancellation, 빠른 종료, late attempt event, graceful/force/tree/reap, output pressure, protocol progress boundary.
- consumer regression: `acp-agent-core`, `git-core`, `workbench-core`, `workbench-host`, AW server process tests.
- platform jobs: macOS Apple Silicon, Linux x86_64, Windows x86_64에서 동일 contract matrix.
- source inventory gate: production `Command::new`/spawn callsite를 정본 inventory와 대조하고 allowlist 밖 직접 spawn을 실패시킨다.

**Target Platform**: macOS 14+ Apple Silicon, supported Linux desktop/server x86_64, Windows 11 x86_64. target별 실제 containment/crash tests를 완료해야 feature를 완료한다.

**Project Type**: reusable Rust infrastructure crate + standalone local server + thin desktop compatibility consumer.

**Performance Goals**:
- status/cancel/shutdown은 100 MiB output pressure 중에도 2초 안에 접수된다.
- short helper supervisor overhead p95 25 ms 이하(실제 command 실행 시간 제외).
- 기본 display-log retained memory는 process별 stream별 상한 안, 전체 server 상한 안에 유지한다.

**Constraints**:
- 외부 공개 순서: domain/containment reserve → spawn → containment adopt → publication CAS+outbox → accepted/started.
- unpublished lease는 durable publication resolver가 판정하며, caller drop만으로 published child를 죽이거나 unpublished child를 남기지 않는다.
- bare PID kill 금지. attempt/keeper control channel 또는 Job handle로만 종료한다.
- protocol bytes는 성공처럼 truncate/drop 금지.
- stdout/stderr 동시 drain, direct child/keeper wait 보장.
- env value/credential logging 금지.
- 044 daemon bootstrap, readiness, drain/stop, desktop quit semantics 유지.

**Scale/Scope**:
- production server-owned spawn families 6개: ACP runner, ACP terminal, Git core/providers, watcher Git helper, catalog curl, login-shell probe.
- 별도 분류 4개: daemon bootstrap, desktop native launcher, build-time command, tests/other apps.
- 동시에 100 supervised attempts와 process별 100 MiB output pressure fixture.

## Constitution Check

*GATE: architecture/constitution alignment는 PASS. feature readiness와 production consumer migration은 platform feasibility evidence 전까지 PENDING.*

- **Monorepo Boundary First**: PASS. 재사용 supervisor는 `crates/process-supervisor`, 서버 조립은 `crates/workbench-host`와 `apps/agentic-workbench-server`, Tauri entry wiring만 AW app에 둔다. 앱 간 import가 없다.
- **Feature-Sliced Frontend Architecture**: N/A. 새 frontend/UI가 없다.
- **Hexagonal Tauri Backend Architecture**: PASS. `ProcessRunner` port와 lifecycle type은 공통 crate의 public contract, OS spawn/signal/pipe는 platform infrastructure다. Tauri는 internal keeper mode 진입과 기존 server bootstrap만 조립한다.
- **Shared Core Before Shared UI**: PASS. 공유 대상은 pure lifecycle/state contract와 infrastructure이며 UI 공유가 없다.
- **Atomic Cross-App Verification**: PASS. `acp-agent-core`, `git-core`, `workbench-core`, `workbench-host`, AW Tauri/server consumer를 모두 검증한다. 다른 앱의 direct spawn은 inventory상 별도 범위이고 새 crate를 소비하지 않는다.
- **Documentation and Storybook**: PASS. `docs/workbench-process-supervision.md`와 045 contracts/inventory를 갱신한다. UI가 없어 Storybook은 N/A다.
- **Testing and Safety**: PENDING. owner/attempt race, credential sentinel, PID reuse 대조, protocol corruption, process-tree crash fixture와 세 platform matrix를 명시했지만 실제 target spike 결과는 아직 없다.

## Project Structure

### Documentation (this feature)

- `specs/045-process-supervisor/spec.md`
- `specs/045-process-supervisor/plan.md`
- `specs/045-process-supervisor/research.md`
- `specs/045-process-supervisor/data-model.md`
- `specs/045-process-supervisor/quickstart.md`
- `specs/045-process-supervisor/checklists/requirements.md`
- `specs/045-process-supervisor/review-ledger.md`
- `specs/045-process-supervisor/contracts/process-lifecycle.md`
- `specs/045-process-supervisor/contracts/output-policy.md`
- `specs/045-process-supervisor/contracts/process-inventory.md`
- `specs/045-process-supervisor/contracts/platform-containment.md`
- `specs/045-process-supervisor/tasks.md` (`/speckit-tasks`에서 생성)

### Source Code (repository root)

- `crates/process-supervisor/src/lib.rs`: `ProcessSupervisor`, `ProcessRunner` port.
- `crates/process-supervisor/src/spec.rs`: `ProcessSpec`, owner/attempt, stream/termination policies.
- `crates/process-supervisor/src/state.rs`: lifecycle reducer와 terminal outcome.
- `crates/process-supervisor/src/registry.rs`: child ownership, cancel/shutdown arbitration, identity ledger.
- `crates/process-supervisor/src/output.rs`: display/capture/protocol stream policies.
- `crates/process-supervisor/src/blocking.rs`: short helper용 bounded blocking facade.
- `crates/process-supervisor/src/platform/unix.rs`: keeper control, process group, descendant identity inventory, wait.
- `crates/process-supervisor/src/platform/windows.rs`: suspended spawn + Job Object.
- `crates/process-supervisor/tests/fixtures/`: tree, leader-exit, session/group escape, double-fork, signal-ignore, output, protocol fixture.
- `crates/acp-agent-core/src/infrastructure/acp/runner.rs`: reserve/spawn/adopt 뒤 Started, protocol stdout.
- `crates/acp-agent-core/src/infrastructure/acp/terminal.rs`: supervised terminal + display output.
- `crates/acp-agent-core/src/infrastructure/acp/util.rs`: supervised login-shell probe/cache.
- `crates/acp-agent-core/src/infrastructure/agent_catalog.rs`: supervised bounded curl capture.
- `crates/git-core/src/git_cli.rs`: injected supervised capture runner.
- `crates/workbench-core/src/infrastructure/git/cli_*_provider.rs`: shared supervised Git runner.
- `crates/workbench-core/src/infrastructure/fs/worktree_watcher.rs`: supervised rev-parse helper.
- `crates/workbench-core/src/infrastructure/orchestration/worktree_guard.rs`: supervised Git capture.
- `crates/workbench-host/src/assembly.rs`: server-wide supervisor injection/shutdown.
- `crates/workbench-host/src/process_keeper.rs`: internal keeper composition hook.
- `apps/agentic-workbench-server/src/main.rs`: keeper mode before normal CLI/serve.
- `apps/agentic-workbench/src-tauri/src/main.rs`: embedded compatibility keeper mode.
- `docs/workbench-process-supervision.md`: 운영 계약.
- `scripts/check-process-spawn-inventory.*`: production direct-spawn gate.

**Structure Decision**: child ownership과 platform primitives를 소비 crate마다 복제하지 않고 독립 `process-supervisor` crate에 둔다. common crate는 workbench domain/storage를 모르며 opaque owner/attempt만 받는다. durable reserve와 accepted/started commit은 domain consumer가 담당한다. `workbench-host`가 server-wide supervisor 한 개를 조립해 모든 consumer에 주입하고 server shutdown에서 `shutdown_all()`을 기다린다.

## 설계 불변식과 구현 순서

1. **Inventory gate부터 고정**: production callsite를 server-owned, daemon bootstrap, desktop-native, build, fixture, other-app으로 분류한다. 누락된 direct spawn이 있으면 구현 전에 표를 고친다.
2. **Lifecycle reducer와 cancellation-safe registry**: `Adopted`에서 publication/activation/abort가 하나의 CAS로 경쟁한다. supervisor는 `UnpublishedProcessLease`를 반환하며, ambiguous storage 상태에서는 child를 quarantine하고 readiness를 내려 winner를 다시 판정한다. Published transaction은 domain result와 outbox를 함께 기록한다.
3. **격리된 platform feasibility spike를 먼저 통과**: Unix keeper EOF 뒤 group kill만 보지 않고 env-clear+exec, leader 조기 종료, 새 group/session, double-fork+reparent, inherited control FD close, keeper-only/server+keeper hard kill, PID reuse TOCTOU에서 안전한 identity/권한을 실제 target에서 확인한다. Windows suspended Job assignment도 같은 descendant leak 기준을 통과해야 한다. 어느 target이든 capability가 입증되지 않으면 production consumer migration을 시작하지 않고 해당 target blocker와 대안을 설계 리뷰로 돌린다.
4. **Output contract**: display log는 bounded drop/truncate + counters, parsed capture는 overflow typed failure, protocol은 exact frame 또는 typed fatal failure다. stdout/stderr drain과 wait는 supervisor가 소유한다.
5. **ACP runner/terminal migration**: prerequisite spike와 lifecycle/outbox fixture가 통과한 뒤 시작한다. `runner.rs`의 현재 Started-before-spawn을 제거한다. `StartAgentRunUseCase`도 background launcher가 시작되기 전에 run 응답을 돌려주는 현재 순서를 분리해, public accepted response가 adoption/publication handshake 뒤에만 나가게 한다. terminal도 durable attempt publish와 registry insert/adopt 전에 create event/reply를 emit하지 않는다.
6. **short helper migration**: catalog curl, login-shell probe, 모든 Git provider와 watcher/orchestration helper를 common runner에 주입한다. sync port 호출은 dedicated blocking boundary에서 실행해 Tokio worker를 막지 않는다.
7. **server shutdown/diagnostics**: drain/force가 process owner policy를 호출하고 마지막에 supervisor가 모든 keeper/job/direct child wait를 보장한다.
8. **inventory gate + platform matrix + 전체 gate**: source scan, consumer suites, crash fixtures, credential sentinel, full workspace validation을 순서대로 실행한다.

## Review Gates

- 이 plan과 Phase 0/1 산출물은 OCR delegate 검토 후 Codex adversarial `--wait` 검토를 같은 tree에서 순차 수행한다.
- 두 리뷰는 Unix nonce 제거/env inventory 권한, identity-check/signal TOCTOU, keeper hard-kill recovery 주체, publication/abort CAS, outbox crash gap, transient domain/durable recovery-anchor 분리가 실제 요구를 충족하는지 명시적으로 판정한다.
- 유효 지적을 반영하고 설계 문서 tree를 다시 고정한 뒤 `/speckit-tasks`로 진행한다.
- 구현 완료 뒤에도 OCR delegate → Codex adversarial `--wait` 순서를 반복하고, 두 리뷰의 findings/verdict와 반영을 별도 ledger에 기록한다.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| 같은 executable의 내부 Unix keeper + identity ledger | macOS에는 parent-death signal이나 Job Object가 없고 process group만으로 새 session/group·double-fork 이탈을 정리할 수 없다 | `kill_on_drop`과 group kill만으로는 server crash, leader 조기 종료, containment 이탈을 처리하지 못한다. bare PID/startup scan은 PID 재사용을 안전하게 판별하지 못하므로 attempt nonce와 start identity가 모두 맞는 live set을 keeper가 종료·재검증한다 |
| Windows native suspended spawn | post-spawn Job assignment 전에 payload가 descendant를 만들 수 있는 race를 제거해야 한다 | 일반 `Command::spawn` 뒤 Job assignment는 reserve→adopt→started 외부 순서는 지켜도 containment 내부 race가 남는다 |
| blocking과 async facade 둘 다 제공 | 기존 Git/domain port는 동기이고 ACP/terminal은 비동기라 한 번에 전부 비동기로 바꾸면 기능 범위가 크게 번진다 | consumer별 직접 `Command` 유지나 별도 wrapper는 shutdown registry와 inventory를 다시 분산시킨다 |

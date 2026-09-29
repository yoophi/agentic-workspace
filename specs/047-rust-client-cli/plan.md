# Implementation Plan: Rust client와 public CLI

**Branch**: `047-rust-client-cli` | **Date**: 2026-09-29 | **Spec**: [spec.md](spec.md)

## Summary

merged044의 wire를 직접 공유하는 headless Rust client와 `aw` CLI를 별도 branch에서 만든다. 현재 host call helper는 full fault outcome/revision/replayed를 보존하지 않고 server runtime dependency도 포함하므로 public client의 기반으로 그대로 호출하지 않는다. pure state/typed reply 및 bounded HTTP/WS/stdio adapter를 독립적으로 구현·검증한다. production launch/ensure·agent profile issuance·signed CLI 배포·실제 desktop/TUI parity는 prerequisites가 없으면 활성화하지 않는다.

## Technical Context

**Language/Version**: Rust edition2021 client/CLI, 기존 Tokio 및 protocol crate.
**Primary Dependencies**: workbench-protocol, serde/serde_json, 기존 lock hyper/http-body-util/hyper-util의 owned HTTP1 connection과 tokio-tungstenite0.24.0 후보. dependency edge는 tasks 단계에서 실제 feature/resolution 확인하며 lock 불필요 갱신 금지. sha2는 기존 lock에 있고 hmac은 없다. 표준 hmac dependency의 compatible version/feature를 구현 setup에서 확인하여 최소한의 정당한 lock 변경만 한다. 자체 crypto 구현은 하지 않는다. browser/Tauri dependency 없음.
**Storage**: 원 data stores에 write 없음. locator descriptor readonly, caller cursor/retry state의 소유권은 caller port. CLI mutation retry는 submission 전 owner-only private runtime-control state로 bound input/key/instance/epoch를 저장한다. user/server data stores에 write하지 않으며 profile install은 gate 뒤.
**Testing**: pure fixtures/golden stdout/stderr, controlled loopback HTTP/WS peer, workbench-host는 dev-only fake engine fixture로 허용. 사용자 data root bootstrap/실제ACP/자동 daemon ensure 및 live user-data mutation 금지. 사용자 지정 actual integration만 merged044 서버binary를 private temp data root에서 명시적으로1회 실행한다.
**Target Platform**: macOS Apple Silicon. 최소14+는 검증 예정이고 현재15 host fixture 근거만 있다. Linux/Windows 제외.
**Performance Goals**: bounded request15s/default connect5s, whole body8MiB, input1MiB, event message/frame1MiB, consumer queue256events/8MiB. configurable nonzero validated limits, 위반은 typed failure. cancellation fixture deadline1s; 실제 app latency는 미검증.
**Project Type**: reusable crate + CLI composition, 이후 TUI/MCP consumer용 port.
**Constraints**: existing-instance-only, no automatic command retry, unknown epoch-bound operation identity, no secret argv/Debug, no production readiness bool 조작.
**Scale/Scope**: current OperationId/OPERATIONS catalog, finite commands+JSONL events. native UI/TUI/MCP server/updater는 이번 기능의 구현이 아닌 원 전체 roadmap의 후속.

## Constitution Check

- Monorepo PASS 설계: crates/workbench-client, apps/aw-cli. 다른 app source import 없음. CLI 및 fixture consumer로 core reuse 검증.
- Frontend N/A: React/UI 변경 없음.
- Hexagonal PASS 설계: domain/ports 순수, application call/event policy, infrastructure HTTP/WS/locator/stdio. host/server runtime은 production dependency가 아님.
- Shared core PASS 설계: UI 없이 pure policy와 typed protocol.
- Atomic verification PASS 계획: protocol 변경 시 protocol/core/server/host/AW affected check; 새 client/CLI core+binary fixtures, pnpm consumer 및 final8gate. 실제 pass는 아직 없음.
- Documentation PASS 계획: docs/workbench-rust-client-cli.md 한국어, Mermaid. Storybook N/A.
- Safety PENDING 구현: endpoint/authority/epoch/outcome/cursor/resource invariants과 prerequisites 검증 필요. design check는 readiness PASS 아님.

## Project Structure

- `crates/workbench-client/src/domain/`: call outcome, attempt identity, limits, cursor/recovery reducer.
- `crates/workbench-client/src/ports/`: credential/verified-endpoint provider, transport and event consumption ports.
- `crates/workbench-client/src/application/`: bounded call and applied cursor/recovery orchestration.
- `crates/workbench-client/src/infrastructure/`: loopback HTTP/WS and readonly locator adapters.
- `apps/aw-cli/src/`: inbound parse, application commands, stdout/stderr adapters, composition only.
- `crates/workbench-protocol/`: existing wire reuse; pure identity proof sharing only if needed after review, no runtime auth/persistence import.
- `specs/047-rust-client-cli/contracts/`: call/connection and CLI/event policy.

## Phases and Dependencies

1. specify→plan→OCR delegate→Codex --wait→findings 적용 후 tasks. 이 단계에서는 implementation/source/library 변경 없음.
2. pure limits/operation classification/full outcome/attempt generation/machine records + tests. independent fixture-only phase, no network/process/storage access.
3. owned HTTP1 TCP connection sender의 identity→handshake/call(재연결 때 proof부터)와 readonly locator + private peer fixtures. client dependency에 workbench-host/runtime 없음. wire identify proof는 원 host와 shared golden conformance를 유지한다. 원 verifier의 HMAC 공식을 복제한 자체 crypto 새 구현 금지: 표준 crate + literal known vectors 및 원 host 비교.
4. event/ticket/snapshot state machine과 JSONL consumer + fake-peer race tests. call/stream tasks 모두 cancellation ownership 및 join 완료 증거를 남긴다. CLI private retry-state의 pre-send durable publish/unknown crash/reopen/old completion CAS를 격리 fixture로 검증한다.
5. **actual merged044 integration**: exact20fcd5f의 실제 server binary를 private temp root에서 실행하고 build source/commit/artifact SHA를 기록한다. Rustclient·실제aw CLI subprocess(`aw events watch --input -`의 반환 streamId 구독/JSONL 출력 포함)로 identity→handshake→project.list/system.describe와 project.create/update/delete·same-key replay를 검증한다. bench.open→orchestration.bootstrap(빈 workspace, Main만 생성)→bootstrap 응답 eventStreamId(`orchestration:<bindingId>`)로 ticket/WS hello→orchestration.recover(empty workspace 한정)→실제 orchestration event 소비/ACK 및 snapshot revision을 대조한다. 원 recover는 scheduler/command/notification reconciliation도 호출한다. Main1/currentRunId null, tasks/generations/reports/commands/notifications/dispatch0, in-flight0를 확인한 private fixture에서만 test-only trigger authority로 실행하고 production generic recover는 gate를 유지한다. service.reconcile_runtime→persist_mutation이 revisionr+1/runtimeReconciled event를 emit하며 dispatcher의 empty next_delivery는 worker queue 전에 끝난다. 후속 notificationRecovery는 저장 revision을 올리지 않고 같은 r+1에 별도 stream sequence를 발행한다. contracts/client.md의 D-C2 acceptance대로 bootstrap s→runtimeReconciled s+1→notificationRecovery s+2를 각각 exact 식별·ACK하고, HTTP reply의 도착 순서와 독립적으로 두 이벤트 처리 후 최종 snapshot r+1을 확인한다. 같은 revision 기반 event dedup/임의 sleep/느슨한 >= 검사를 금지한다. 이 분기와 actual run0을 증거로 검증한다. project mutation이 event를 emit한다고 가정하지 않고 원 orchestration event producer를 사용한다. test server 시작과 CLI existing-instance 연결을 구분하며 CLI가 daemon을 자동 시작하는 횟수0. fixture process guard는 startup/cancel/panic/error에서도 child ownership을 유지한 bounded kill/try_wait/reap와 stderr/exit/cleanup 결과를 기록한다. fake peer는 adversarial race 전용, actual server wire pass 없이는 client adapter 완료를 선언하지 않는다. live user-data/ACP/terminal/Git helper/catalog curl/PATH probe invoke 없음. source 변경 뒤 oldserver binary 출처를 새server build로 위장하지 않는다.
6. production adoption와 public activation는 아래 gate가 허용한 operation에만 연결한다. missing prerequisite를 CLI generic call/명시 command로 우회할 수 없다. actual peer test는 기능 완료와 구분한다.
7. 최신 사용자 지시(2026-09-29): 047 자체 계약/actual server 검증을 마친 뒤 final implementation OCR delegate→Codex adversarial --wait→valid findings 수정/재검증→PR CI/squash/main sync를 수행한다. 045/046·TUI/MCP·배포·fallback 제거는 미완료/gate 유지/후속 재개 조건을 인계하며 구현을 시작하지 않는다.
8. `docs/047-completion-handoff.md`에 완료범위/commit·PR/검증근거/이연 prerequisites/후속작업/재개방법을 기록하고 중지한다. 이는 전체 전환 완료가 아니다. T016 production exit6의 외부 전제 미충족도 명시적으로 리뷰하며 활성화하지 않는다.

## Branch / Base / 045·046 dependencies

base `20fcd5fdcf633ae06792d51a9b963e3857909440` = origin/main merged044(#208). 046 `9b1b2e2` 및045 `6e4bf30` branch preserved, cherry-pick/삭제 없음. base 대비046의 `crates/workbench-protocol`, `crates/workbench-host/src/lifecycle`, `packages/workbench-client` diff0files를 실제 확인했다. 따라서 wire/caller fixture 설계는045/046 코드 readiness를 전제로 하지 않는다. 046 Cargo.lock의 serialize 관련10줄 차이는 새branch로 가져오지 않았다. unrelated untracked `docs/code-review-app-migration.md` 제외.

| 작업 | 독립성 / 실제 gate |
|---|---|
| typed policy/finite CLI output/controlled peer | main wire 기반 독립. production migration/freeze/child 소유 없음 |
| verified 기존 서버 조회 | readonly locator와 identity/handshake/permission 증거 필요. 새 daemon/lease 자동 ensure 금지 |
| 실제 격리 서버의 비실행 operation | merged044 wire integration은 project CRUD, bench.open, orchestration.bootstrap/get 및 empty-workspace recover(test harness 한정) closed allowlist이며 source에서 launch 없음 확인. 원 scopes/revision/idempotency 검사 및 temp-root 한정. 자동spawn/launch gate 우회 없음 |
| agent/run/terminal/Git/helper를 실행하거나 migration/freeze/ensure/stop하는 operation | 해당 원 scopes/revision/grant뿐 아니라 새 CLI production activation에045 containment와046 lifecycle/data prerequisites 필요. 현재 server가 이 readiness proof를 제공한다고 가정하지 않음. 미연결 경로는 PrerequisiteUnavailable로 거절하며 generic call도 동일 policy |
| actual desktop 종료 후 run 관찰/취소, packaging/updater |045 T010/T016,046 actual migration/backup/restore, macOS sign/notarization/install, compatible executable discovery 증거 뒤. fixture로 완료처리 금지 |
| TUI/MCP host matrix |원 roadmap6/7 후속. 새 client port를 쓸 수 있으나 UI/protocol compatibility 완료 아님 |

## Constitution Recheck / Readiness

구조 설계 위반 없음, actual safety/readiness PENDING. extensions.yml과 update-agent-context.sh는 이 base에 없으며 hook/context script 미실행 이유를 기록한다. 원 전체 목표는 superseded 이력과 후속 인계 범위로 보존한다. 최신 047 종료 gate는 가능한 client/CLI 계약과 exact merged044 통합·최종 검증/순차 리뷰/PR·CI/merge/main sync/인계 기록이며, 미완료 후속 roadmap 구현을 047 merge 전제로 요구하지 않는다. production gate와 지원/보장 수준은 실제 증거 없이 확대하지 않는다.

## 구현 checkpoint와 잔여 종료 gate

위 Constitution Check의 설계/계획 PASS와 당시 미실행 설명은 이력이다. 현재 T001–T039 controlled 및 actual exact merged044 wire checkpoint의 실제 실행 증거는 validation.md에 기록했다. 최종 affected/root8 검증·순차 구현 리뷰·PR/CI·main merge/sync·인계는 완료 전이며 production readiness를 주장하지 않는다. 045/046 및 설치본/macOS14+/desktop 종료 후 run/TUI/MCP/signing/update/fallback 제거는 후속 미완료다.

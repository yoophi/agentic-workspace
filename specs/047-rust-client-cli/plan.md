# Implementation Plan: Rust client와 public CLI

**Branch**: `047-rust-client-cli` | **Date**: 2026-09-29 | **Spec**: [spec.md](spec.md)

## Summary

merged044의 wire를 직접 공유하는 headless Rust client와 `aw` CLI를 별도 branch에서 만든다. 현재 host call helper는 full fault outcome/revision/replayed를 보존하지 않고 server runtime dependency도 포함하므로 public client의 기반으로 그대로 호출하지 않는다. pure state/typed reply 및 bounded HTTP/WS/stdio adapter를 독립적으로 구현·검증한다. production launch/ensure·agent profile issuance·signed CLI 배포·실제 desktop/TUI parity는 prerequisites가 없으면 활성화하지 않는다.

## Technical Context

**Language/Version**: Rust edition2021 client/CLI, 기존 Tokio 및 protocol crate.
**Primary Dependencies**: workbench-protocol, serde/serde_json, 기존 lock hyper/http-body-util/hyper-util의 owned HTTP1 connection과 tokio-tungstenite0.24.0 후보. dependency edge는 tasks 단계에서 실제 feature/resolution 확인하며 lock 불필요 갱신 금지. standard HMAC implementation은 기존 lock hmac/sha2를 검토한다. browser/Tauri dependency 없음.
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
5. **actual merged044 integration**: exact20fcd5f의 실제 server binary를 private temp root에서 실행하고 build source/commit/artifact SHA를 기록한다. Rustclient·실제aw CLI subprocess로 identity→handshake→project.list/system.describe와 project.create/update/delete·same-key replay를 검증한다. bench.open→orchestration.bootstrap(빈 workspace, Main만 생성)→ticket/WS hello→orchestration.setPresentation(Main 상태만)→실제 orchestration event 소비/ACK 및 snapshot revision을 대조한다. 원 runtime bootstrap/set_presentation는 service의 blocking JSON mutation이고 worker launch 호출이 없음을 actual source에서 확인했다. project mutation이 event를 emit한다고 가정하지 않고 원 orchestration event producer를 사용한다. test server 시작과 CLI existing-instance 연결을 구분하며 CLI가 daemon을 자동 시작하는 횟수0. fixture process guard는 startup/cancel/panic/error에서도 child ownership을 유지한 bounded kill/try_wait/reap와 stderr/exit/cleanup 결과를 기록한다. fake peer는 adversarial race 전용, actual server wire pass 없이는 client adapter 완료를 선언하지 않는다. live user-data/ACP/terminal/Git helper/catalog curl/PATH probe invoke 없음. source 변경 뒤 oldserver binary 출처를 새server build로 위장하지 않는다.
6. production adoption와 public activation는 아래 gate가 허용한 operation에만 연결한다. missing prerequisite를 CLI generic call/명시 command로 우회할 수 없다. actual peer test는 기능 완료와 구분한다.
7. final implementation OCR→Codex→validation→PR CI/squash/main sync. 047 전체 readiness가 없으면 foundation-only로 merge-ready를 축소하지 않는다.

## Branch / Base / 045·046 dependencies

base `20fcd5fdcf633ae06792d51a9b963e3857909440` = origin/main merged044(#208). 046 `9b1b2e2` 및045 `6e4bf30` branch preserved, cherry-pick/삭제 없음. base 대비046의 `crates/workbench-protocol`, `crates/workbench-host/src/lifecycle`, `packages/workbench-client` diff0files를 실제 확인했다. 따라서 wire/caller fixture 설계는045/046 코드 readiness를 전제로 하지 않는다. 046 Cargo.lock의 serialize 관련10줄 차이는 새branch로 가져오지 않았다. unrelated untracked `docs/code-review-app-migration.md` 제외.

| 작업 | 독립성 / 실제 gate |
|---|---|
| typed policy/finite CLI output/controlled peer | main wire 기반 독립. production migration/freeze/child 소유 없음 |
| verified 기존 서버 조회 | readonly locator와 identity/handshake/permission 증거 필요. 새 daemon/lease 자동 ensure 금지 |
| 실제 격리 서버의 비실행 operation | merged044 wire integration은 project CRUD, bench.open, orchestration.bootstrap/get/setPresentation(Main only) closed allowlist이며 source에서 launch 없음 확인. 원 scopes/revision/idempotency 검사 및 temp-root 한정. 자동spawn/launch gate 우회 없음 |
| agent/run/terminal/Git/helper를 실행하거나 migration/freeze/ensure/stop하는 operation | 해당 원 scopes/revision/grant뿐 아니라 새 CLI production activation에045 containment와046 lifecycle/data prerequisites 필요. 현재 server가 이 readiness proof를 제공한다고 가정하지 않음. 미연결 경로는 PrerequisiteUnavailable로 거절하며 generic call도 동일 policy |
| actual desktop 종료 후 run 관찰/취소, packaging/updater |045 T010/T016,046 actual migration/backup/restore, macOS sign/notarization/install, compatible executable discovery 증거 뒤. fixture로 완료처리 금지 |
| TUI/MCP host matrix |원 roadmap6/7 후속. 새 client port를 쓸 수 있으나 UI/protocol compatibility 완료 아님 |

## Constitution Recheck / Readiness

구조 설계 위반 없음, actual safety/readiness PENDING. extensions.yml과 update-agent-context.sh는 이 base에 없으며 hook/context script 미실행 이유를 기록한다. plan/spec가 pure foundation과 gated integration을 모두 추적하며 전체 목표·지원/보장 수준을 줄이지 않는다.

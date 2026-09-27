# Implementation Plan: 작업대(Bench) 도입과 run·교환 command 이관 (2b-1)

**Branch**: `040-workbench-owners` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/040-workbench-owners/spec.md` (grill 확정 Q1–Q10, ADR 5건)

## Summary

core에 **작업대(Bench)** registry를 두고, run 기계를 객체 안전한 `RunEngine` 포트 뒤로 옮겨 run 8개·교환 4개 command와 MCP 교환·제목 도구를 `Workbench.call` operation 18개로 이관한다. 작업대는 연 principal 주체에 묶이고, 모든 run·교환 제어는 "대상 run이 이 작업대 소유인가"를 검사한다. 데스크톱은 창 label ↔ 작업대 대응표만 갖고, 발행 결과를 `DesktopBridge` 포트로 받아 창 삽입 경로 하나로 전달한다(네이티브 방송 제거). `run.start`만 변경 기록(재시작 `pending` → `unknown`), 나머지는 세대 범위 멱등성. 교환 스트림(상태 복원용)·작업대 스트림(알림용)을 연다. orchestration(041)은 과도기 접근자로 같은 run 기계와 작업대를 쓴다. 상세 결정은 [research.md](research.md) R1–R15.

## Technical Context

**Language/Version**: Rust 1.98(workbench-* edition 2021, AW edition 2024), TypeScript 5.x

**Primary Dependencies**: 기존(tokio, serde, utoipa 5, axum 0.7, rusqlite, uuid, acp-agent-core) — 새 의존성 없음

**Storage**: SQLite ledger(`run.start`만 새 행 종류), `acp-sessions.json`(AW → core로 저장소 코드 이동, 형식 불변). 작업대·run 소유·교환·세대 멱등 표는 메모리

**Testing**: cargo test(call fixture `steps` 확장, 이벤트 fixture, 흐름 테스트, reconciler, `ScriptedRunEngine`), vitest(workbench-client `EventMap`·`OperationMap` test-d), AW compat·bridge 단위 테스트, `#[ignore]` 지연 측정

**Target Platform**: macOS desktop(Tauri 2.11), 테스트 loopback

**Project Type**: desktop app + shared crates(monorepo)

**Performance Goals**: `run.sendPrompt` 경유 지연 증가 p95 < 5ms(R14), 데스크톱 이벤트 전달 순서 = 순번 순서(039 유지)

**Constraints**: 화면 코드 diff 0 목표, `crates/acp-agent-core`·`packages/agent-client` diff 0, orchestration 동작 불변(041), 오류 문구 유지(새 검사 5종 제외), 창 label은 AW 대응표 밖에 없음

**Scale/Scope**: operation 32 → 50, scope 14 → 20, 스트림 kind 구독 가능 2 → 4, Tauri command 이관 12개(누적 45), 이연 18(orchestration)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 계약 `crates/workbench-protocol`(operation·scope·principal·이벤트 DTO), 구현 `crates/workbench-core`(작업대·run·교환 서비스, `RunEngine`·`DesktopBridge` 포트, 운영 `AcpRunEngine`, `JsonAcpSessionStore` 이동), 데스크톱 어댑터 `apps/agentic-workbench/src-tauri`(compat command, `DesktopBenches`, `TauriDesktopBridge`, MCP 도구 호출 경로), 생성물 `packages/workbench-client`. 앱 간 import 없음.
- **Feature-Sliced Frontend Architecture**: PASS(N/A 목표) — 화면 변경 없음. 수신 어댑터는 이미 삽입 경로를 듣는다(research 사실 요약).
- **Hexagonal Tauri Backend Architecture**: PASS — 도메인·서비스는 core application, 엔진·저장은 core infrastructure, 외부 효과는 core ports(`RunEngine`, `DesktopBridge`, `RunTerminalHook`, `RunLaunchDecorator`). AW command는 입력 변환 → `Workbench.call` → 출력·오류 변환만 한다. Tauri 전달·MCP transport·창 대응은 AW inbound/infrastructure.
- **Shared Core Before Shared UI**: PASS — 순수 core만 공유.
- **Atomic Cross-App Verification**: PASS — `crates/workbench-*` 소비자는 AW 하나. `acp-agent-core`·`packages/agent-client` 불변(hushline·ask-code 영향 없음, quickstart §2). `cargo test --workspace`·`pnpm run check-types`로 전체 확인.
- **Documentation and Storybook**: PASS — `docs/workbench-seam.md`(인벤토리·작업대·스트림·MCP 절), 정본 진행 각주, ADR 5건 작성 완료. Storybook N/A.
- **Testing and Safety**: PASS — 작업대 주체 검사·run 소유 검사(교차 작업대·교차 주체 fixture), 경로 정규화(`bench.open`), 권한 응답 소유, 교환 확인 멱등, MCP agent principal 최소 scope, `run.start` 재시작 판정, 세대 멱등 충돌·동시성 테스트.

**Post-design re-check (Phase 1 뒤)**: 변동 없음 — 전부 PASS. Q9 구현 수준 변경(가짜를 `SessionLauncher` 대신 `RunEngine` 수준에 주입, research R3)은 의도가 같고 acp-agent-core 불변을 지키기 위한 것이다.

## Project Structure

### Documentation (this feature)

```text
specs/040-workbench-owners/
├── spec.md
├── plan.md
├── research.md          # R1–R15
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── workbench-benches.md
│   └── tauri-compat.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/workbench-protocol/src/
├── principal.rs            # PrincipalKind::Agent, subject, scope 6개, agent()
├── call.rs                 # OperationId 18개
├── descriptor.rs           # idempotencyScope
├── operations/
│   ├── bench.rs            # 신규: open/close/requestTitle input·output
│   ├── run.rs              # 신규: 8개 input, AgentRunRequest/AgentRun/후보 DTO 미러
│   └── exchange.rs         # 신규: 7개 input, 교환 DTO 미러
├── events/
│   ├── mod.rs              # StreamKind::Bench, Exchange 구독 가능, 스키마 1개 추가·body_schema
│   ├── exchange.rs         # 신규: ExchangeRequestedDto, AgentExchangeDto
│   └── bench.rs            # 신규: TitleRequestedDto
└── openapi.rs              # component·variant 추가

crates/workbench-core/src/
├── application/
│   ├── bench_service.rs            # 작업대 수명·주체 검사
│   ├── run_service.rs              # run operation, 소유 검사
│   ├── exchange/                   # AW에서 이동: domain·service (키 = BenchId)
│   ├── epoch_idempotency.rs        # 세대 범위 멱등 표
│   ├── handlers/{bench,run,exchange}/
│   ├── reconcilers/run_start.rs    # pending → unknown
│   └── workbench_runtime.rs        # 어댑터·registry 조립, run_sink(bench) 과도기 접근자
├── infrastructure/
│   ├── bench/in_memory_bench_registry.rs
│   ├── exchange/in_memory_workspace_registry.rs   # AW에서 이동
│   ├── run/{acp_run_engine.rs, workbench_run_sink.rs}
│   ├── fs/acp_session_store.rs     # AW JsonAcpSessionStore 이동
│   └── event_hub/                  # remove_stream, publish_notification(deliver)
└── ports/{run_engine.rs, desktop_bridge.rs}

crates/workbench-core/tests/
├── support/{scripted_run_engine.rs, recording_desktop.rs, fixtures steps}
├── bench_run_flow.rs, bench_isolation.rs, exchange_flow.rs, epoch_idempotency.rs, run_start_reconcile.rs
└── (기존) contract_suite.rs, event_contract_suite.rs

crates/workbench-protocol/fixtures/{bench-*,run-*,exchange-*}.json, fixtures/events/{exchange-*,bench-*}.json

apps/agentic-workbench/src-tauri/src/
├── inbound/tauri_commands.rs       # 12개 command → compat
├── inbound/workbench_compat.rs     # 교환 오류 JSON, run 입력 변환
├── infrastructure/desktop_benches.rs        # 신규: 창 ↔ 작업대
├── infrastructure/tauri_desktop_bridge.rs   # 신규: DesktopBridge, RunTerminalHook, RunLaunchDecorator 구현
├── infrastructure/mcp/{mod.rs, agent_exchange_tool.rs, title_tool.rs}  # agent principal → Workbench.call
├── lib.rs                          # AppState·교환 registry manage 제거, 창 닫힘 → bench.close
└── (삭제) domain/agent_exchange.rs, application/agent_exchange_service.rs, application/agent_tool_candidate_service.rs,
           application/mcp_title_control_service.rs, infrastructure/in_memory_agent_workspace_registry.rs,
           infrastructure/json_acp_session_store.rs, infrastructure/tauri_run_event_sink.rs, ports/agent_workspace_registry.rs

packages/workbench-client/src/   # 생성물 + OperationMap/EventMap alias·test-d
docs/workbench-seam.md, docs/client-server-architecture-research.md
```

**Structure Decision**: 037–039의 3계층(protocol 계약 / core 구현 / AW 호환 어댑터)을 그대로 쓴다. 교환은 038 도메인 이관 절차(도메인·서비스·registry 이동 → operation·DTO·fixture → compat → AW 파일 삭제)를 따른다. orchestration이 쓰는 run 기계는 core가 소유하고 AW는 과도기 접근자로 빌린다(R12, 041에서 제거).

## Complexity Tracking

| 항목 | 이유 | 더 단순한 대안을 택하지 않은 이유 |
|---|---|---|
| `RunEngine` 포트(유스케이스 위 한 겹) | `AppState`의 `Session`이 `AcpSession`으로 고정되어 가짜 launcher를 주입할 수 없다 | `AppState` 제네릭화는 acp-agent-core 변경(hushline·ask-code 파급) |
| 과도기 접근자 `acp_registry()`·`run_sink(bench)` | orchestration(041)이 같은 run 기계·작업대를 써야 창 닫힘·전달이 하나로 유지된다 | orchestration을 040에 포함하면 Q1(분할) 위배 |
| 멱등 규칙 두 종류(durable·epoch) | 메모리 상태에 영속 기록은 거짓 보장(ADR core 0005) | 하나로 통일하면 프롬프트마다 SQLite 쓰기 또는 재시도 중복 |

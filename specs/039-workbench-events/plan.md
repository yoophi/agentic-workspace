# Implementation Plan: 이벤트 모델 통합 — Workbench 이벤트 스트림 (2a)

**Branch**: `039-workbench-events` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/039-workbench-events/spec.md` (grill 확정 Q1–Q9, ADR 4건)

## Summary

`Workbench.events`를 실제로 구현한다. core에 `EventHub`를 두어 스트림별 lock 하나로 발행·데스크톱 전달과 구독(등록 → high-water → replay → drain)을 직렬화하고, 보관 한도로 지운 run은 제거 표식으로 시작 전 run과 구별하며, 세대·gap·한도를 계약으로 고정한다. AW의 run journal을 hub로 옮겨 run 이벤트가 live·replay에서 같은 번호를 갖게 하고, 데스크톱에는 번호를 실은 봉투를 창 삽입 경로 하나로 보낸다. worktree 감시는 core로 옮겨 실제 경로별 참조 수로 공유하는 알림 스트림이 되고, 데스크톱 watcher command 2개는 구독 task로 바뀐다. 이벤트 스키마는 registry에서 OpenAPI·TS `EventMap`으로 생성되고, 테스트 HTTP 하네스는 WebSocket으로 같은 구독을 검증한다. 상세 결정은 [research.md](research.md) R1–R15.

## Technical Context

**Language/Version**: Rust 1.98(workbench-* edition 2021, AW edition 2024), TypeScript 5.x(React 19)

**Primary Dependencies**: 기존(tokio, serde, utoipa 5, axum 0.7, notify 6) + `futures-core`(protocol), 테스트 전용 `tokio-tungstenite`·`futures-util`, axum `ws` feature(dev)

**Storage**: 없음. journal은 메모리(ADR core 0002). ledger·JSON 저장소 변경 없음

**Testing**: cargo test(unit·integration·race·fixture), vitest(run controller), vitest typecheck(`EventMap`), `#[ignore]` latency

**Target Platform**: macOS desktop(Tauri 2.11), 테스트 loopback

**Project Type**: desktop app + shared crates(monorepo)

**Performance Goals**: publish→구독자 수신 p95 증가 < 10ms(SC-006), race test 1,000회 누락·중복 0(SC-001)

**Constraints**: 프론트 변경은 `features/agent-run`·`entities/agent-run/api|model`만. acp-agent-core·`@yoophi/agent-client` 불변. orchestration·exchange·창 제목·외관 이벤트 경로 불변. 창 닫힘 = run 취소 불변

**Scale/Scope**: 스트림 kind 2개 구독(run, worktree), 예약 2개(orchestration, exchange). 한도: run당 512, 보관 run 256, 구독자 대기열 1,024, 동시 구독 256

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 공통 계약은 `crates/workbench-protocol`, 구현은 `crates/workbench-core`, 데스크톱 전달은 `apps/agentic-workbench/src-tauri`, 화면은 `apps/agentic-workbench/src`, 생성 타입은 `packages/workbench-client`. 앱 간 import 없음. 공유 패키지 `@yoophi/agent-client` 타입은 바꾸지 않고 AW에서 확장(research R6·R7).
- **Feature-Sliced Frontend Architecture**: PASS — 수신 어댑터·타입은 `entities/agent-run/{api,model}`, 재수화·live 처리는 `features/agent-run/ui`. 다른 레이어 변경 없음(SC-008 grep).
- **Hexagonal Tauri Backend Architecture**: PASS — `EventHub`는 core infrastructure, AW sink가 의존하는 `RunEventPublisher`는 core ports, Tauri 전달(삽입·`emit_to`)은 AW infrastructure/inbound에 남는다. command는 입력 변환 → `Workbench.events`/publisher 호출만 한다.
- **Shared Core Before Shared UI**: PASS — 순수 core만 공유. 공유 UI 없음.
- **Atomic Cross-App Verification**: PASS — `crates/*` 변경 소비자는 AW 하나(`workbench-*`). acp-agent-core·`packages/agent-client`는 불변(hushline·ask-code 영향 없음, quickstart §2 grep). 전체 `cargo test --workspace`와 `pnpm run check-types`로 확인.
- **Documentation and Storybook**: PASS — `docs/workbench-seam.md`에 이벤트 계약·구독 순서·한도·분류·데스크톱 전달 절 추가, 정본 진행 각주 갱신. ADR 4건은 grill에서 작성 완료. Storybook N/A.
- **Testing and Safety**: PASS — race test·fixture(in-memory·WS)·worktree 참조 수·lag·epoch·retention 테스트. worktree 스트림은 실제 경로 정규화와 오늘의 제외 디렉터리 규칙을 core에서 적용. run·permission 소유 범위는 변경 없음(2b).

**Post-design re-check (Phase 1 뒤)**: 변동 없음 — 전부 PASS. data-model의 포트 배치(`ports/event_publisher.rs`)는 037·038의 top-level `ports` 관례와 같다.

## Project Structure

### Documentation (this feature)

```text
specs/039-workbench-events/
├── spec.md
├── plan.md
├── research.md          # R1–R15
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── workbench-events.md
│   └── tauri-compat-events.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/workbench-protocol/src/
├── workbench.rs          # EventStream(Stream 래퍼)·EventItem·GapNotice
├── events/               # 신규: mod.rs(EVENT_SCHEMAS registry, EventFrame), run.rs(RunEventDto), worktree.rs, orchestration.rs
├── descriptor.rs         # DescribeOutput + epoch, eventSchemas
├── principal.rs          # Scope::RunRead
└── openapi.rs            # EventBySchema·EventFrame 조립

crates/workbench-core/src/
├── application/
│   ├── workbench_runtime.rs      # epoch, events() 구현, RunEventPublisher 구현
│   └── event_dto.rs              # RunEvent ↔ RunEventDto wire 동일성 테스트
├── infrastructure/
│   ├── event_hub/                # 신규: mod.rs(EventHub), stream.rs(StreamState·cursor 판정), subscription.rs(EventStream 구현)
│   └── fs/worktree_watcher.rs    # AW fs_worktree_watcher.rs에서 이동
└── ports/event_publisher.rs      # 신규

crates/workbench-core/tests/
├── event_subscription_race.rs
├── run_delivery_order.rs          # 동시 발행자 전달 순서(research R6)
├── event_contract_suite.rs       # fixture × (in-memory, WS)
├── worktree_stream.rs
├── event_latency.rs              # #[ignore]
└── support/http_harness.rs       # GET /v1/events WebSocket 추가

crates/workbench-protocol/fixtures/events/*.json

apps/agentic-workbench/src-tauri/src/
├── infrastructure/tauri_run_event_sink.rs    # publish_run + 삽입 전달
├── inbound/tauri_commands.rs                 # replay command·watcher command 교체
├── lib.rs                                    # journal manage 제거
└── (삭제) infrastructure/in_memory_runtime_event_journal.rs, ports/runtime_event_journal.rs, infrastructure/fs_worktree_watcher.rs

apps/agentic-workbench/src/
├── entities/agent-run/model/types.ts          # DeliveredRunEvent
├── entities/agent-run/api/agent-run-repository.ts
└── features/agent-run/ui/agent-run-runtime-host.tsx (+ model 테스트)

packages/workbench-client/src/   # 생성물 + EventMap alias·test-d
docs/workbench-seam.md, docs/client-server-architecture-research.md
```

**Structure Decision**: 037·038의 3계층(protocol 계약 / core 구현 / AW 호환 어댑터)을 그대로 쓴다. 이벤트 계약은 operation과 같은 protocol crate의 `events` 모듈에 두어 describe·OpenAPI·TS가 한 registry에서 생성되게 한다.

## Complexity Tracking

위반 없음. 기록할 복잡도 1건:

| 항목 | 이유 | 더 단순한 대안을 택하지 않은 이유 |
|---|---|---|
| 데스크톱이 구독자가 아닌 "발행 결과 전달"(세 경로 중 하나가 다른 경로) | 창↔run 연결은 2b의 창 정체 분해 대상 | 창마다 구독하면 2b 작업을 앞당겨야 함(ADR 0003) |

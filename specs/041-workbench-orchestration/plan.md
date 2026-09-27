# Implementation Plan: orchestration을 작업대 기준으로 이관 (서버-클라이언트 전환 2b-2)

**Branch**: `041-workbench-orchestration` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/041-workbench-orchestration/spec.md`

## Summary

AW에 남은 orchestration(도메인·서비스·저장소·scheduler·알림 전달·자식 run 기동·worktree 감시, 약 7,500줄)을 `workbench-core`로 옮기고, 소유를 창 label에서 작업대 묶임(메모리)으로 바꾼다. orchestration 화면 동작 18개(`orchestration.*` 17 + `run.replay`)와 MCP orchestration 도구 16개(agent 전용 operation 16)를 `Workbench.call`로 제공하고(operation 50 → 84), 묶임마다 새 id를 갖는 `orchestration:<bindingId>` 스트림을 연다. 저장소는 파일 하나이므로 **저장소 전체의 읽기-수정-쓰기를 `StorageCoordinator` aggregate lock 하나로 직렬화**하고(서로 다른 작업 영역의 동시 변경 보호), await가 낀 다단계 흐름은 작업 영역별 async lock으로 순서를 맞춘다(research R1·R2). agent 권한은 토큰 주장이 아니라 서버 상태로 판정한다(R7). run 스트림에 소유 작업대를 기록해 `run.replay`와 run 구독의 권한을 검사한다(R17). 040의 과도기 통로와 AW의 orchestration 후처리·창 닫힘 해제를 모두 없애 core·계약에서 창 label을 0으로 만든다(R14). 화면 코드는 바뀌지 않는다 — AW 호환 command가 `boundWindowLabel`을 다시 채운다(R11).

## Technical Context

**Language/Version**: Rust 2021(workspace 기존 toolchain), TypeScript 5(생성 타입만)

**Primary Dependencies**: 기존 — tokio, serde, utoipa, axum 0.7(테스트 HTTP harness), tauri 2, `acp-agent-core`(불변). 새 의존성 없음.

**Storage**: `orchestration-sessions.json`(형식 유지, `boundWindowLabel`만 생략), `StorageCoordinator` aggregate `orchestration-sessions`. ledger 미사용(ADR core 0005 유지).

**Testing**: `cargo test`(core 단위·통합·계약 suite fixture 두 경로), `vitest --typecheck`(workbench-client 타입 테스트), AW 테스트.

**Target Platform**: macOS 데스크톱(AW), core는 플랫폼 무관.

**Project Type**: desktop-app + Rust library crates(서버 core).

**Performance Goals**: orchestration operation의 seam 경유 추가 지연은 040 기준(run 제어 p95 +수십 µs)과 같은 수준. 동시성 테스트 100회 이상에서 손실 0.

**Constraints**: 화면 diff 0, `acp-agent-core`·`packages/agent-client` diff 0, 저장 형식 이전 빌드 호환, 저장소 lock 안에서 await 금지.

**Scale/Scope**: command 18 + MCP 도구 16, AW orchestration 코드 ~7,500줄 이동, AW orchestration 테스트 60개 이동.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 계약 `crates/workbench-protocol`(operation 34·DTO·scope·스트림), 구현 `crates/workbench-core`(orchestration domain/application/infrastructure, 묶임·scheduler·worktree 감시), 데스크톱 어댑터 `apps/agentic-workbench/src-tauri`(compat command, 전달, MCP transport), 생성물 `packages/workbench-client`. 앱 간 import 없음.
- **Feature-Sliced Frontend Architecture**: PASS(N/A 목표) — 화면 변경 없음. 수신은 이미 삽입 경로(research 사실 요약).
- **Hexagonal Tauri Backend Architecture**: PASS — orchestration 도메인은 core domain, 서비스는 core application, 저장·엔진 연동은 core infrastructure, 외부 효과는 core ports(`OrchestrationRepository`·`AgentWorker`·`DesktopBridge`·`RunLaunchDecorator`). AW command는 입력 변환 → `Workbench.call` → 출력·오류 변환만.
- **Shared Core Before Shared UI**: PASS — 순수 core만 공유.
- **Atomic Cross-App Verification**: PASS — `crates/workbench-*` 소비자는 AW 하나. `acp-agent-core`·`agent-client` 불변. `cargo test --workspace`·`pnpm run check-types`·`pnpm run test`(quickstart §1).
- **Documentation and Storybook**: PASS — `docs/workbench-seam.md`(인벤토리 이연 0, orchestration 절, 스트림·권한 규칙), 정본 진행 각주, core ADR 2건(R16). Storybook N/A.
- **Testing and Safety**: PASS — 저장소 전체 동시성(cross-workspace 포함), 작업대 주체·묶임 검사, agent 역할(이전 세대·다른 과제) 검사, run 스트림·replay 소유 검사, worktree 감시, 묶임 스트림 권한, 저장 형식 호환 테스트.

**Post-design re-check (Phase 1 뒤)**: 전부 PASS. 설계 중 추가된 범위 두 가지 — (1) 스트림 id를 묶임별로 둔 것(040 제거 표식과 복구의 충돌 회피, FR-009 갱신), (2) run 스트림 소유 기록(R17, `run.replay` 권한을 판단하려면 필요하고 040이 남긴 run 구독 누수도 닫는다) — 은 constitution 위반이 아니다.

## Project Structure

### Documentation (this feature)

```text
specs/041-workbench-orchestration/
├── spec.md
├── plan.md
├── research.md          # 사실 요약 + R1–R17
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── workbench-orchestration.md
│   └── tauri-compat.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/workbench-protocol/src/
├── principal.rs                    # Scope::OrchestrationRead/Write, AGENT_SCOPES 확장
├── operations/orchestration.rs     # 신규: 입력·DTO(작업 영역·노드·세대·과제·보고·명령·알림·분배)
├── operations/run.rs               # run.replay 입력·RunReplayDto
├── operations/mod.rs               # OPERATIONS 84, schema_for
├── events/mod.rs                   # Orchestration 구독 가능, scope orchestration:read
└── openapi.rs                      # component·golden

crates/workbench-core/src/
├── domain/agent_orchestration.rs           # AW에서 이동(boundWindowLabel 생략·무시)
├── ports/{orchestration_repository,agent_worker,coordinator_notification}.rs   # 이동, read/update 포트
├── application/orchestration/              # 신규 모듈: service·command_service·scheduler·notification_dispatcher·binding·roles·runtime(작업 영역 lock)
├── application/handlers/orchestration/     # 데스크톱 18·agent 16 handler
├── application/handlers/run/               # run.replay
├── infrastructure/fs/orchestration_store.rs    # JsonOrchestrationRepository 이동 + aggregate lock
├── infrastructure/orchestration/engine_agent_worker.rs  # 자식 run 기동(입장·decorator·RunEngine), worktree 감시
├── infrastructure/event_hub/mod.rs         # run 스트림 소유 기록, orchestration journal
└── application/workbench_runtime.rs        # 과도기 접근자 제거, RuntimeAdapters.orchestration

crates/workbench-core/tests/
├── orchestration_concurrency.rs    # cross-workspace·same-workspace 100회+
├── orchestration_flow.rs           # 닫기→복구, worktree 실패, scheduler, 이전 세대 거절
├── orchestration_stream.rs         # 묶임 스트림·구독 권한·run.replay 권한
└── (fixtures) crates/workbench-protocol/fixtures/orchestration-*.json

apps/agentic-workbench/src-tauri/src/
├── inbound/tauri_commands.rs       # 18개 compat(boundWindowLabel 재구성)
├── inbound/workbench_compat.rs     # orchestration 오류·결과 변환
├── infrastructure/tauri_desktop_bridge.rs   # Orchestration 전달, RunTerminalHook 삭제
├── infrastructure/mcp/orchestration_tool.rs # agent principal → Workbench.call
├── infrastructure/mcp/capability_registry.rs# token → run id
├── lib.rs                          # release_window·scheduler·orchestration 조립 제거
└── (삭제) domain/agent_orchestration.rs, application/orchestration_*, coordinator_notification_dispatcher.rs,
          infrastructure/json_orchestration_repository.rs, tauri_orchestration_event_sink.rs, acp_agent_worker_adapter.rs(orchestration 부분), ports/orchestration_*

packages/workbench-client/src/      # 생성 타입, operation-map alias·test-d
docs/workbench-seam.md, docs/client-server-architecture-research.md
crates/workbench-core/docs/adr/0006-*.md, 0007-*.md
```

**Structure Decision**: 040과 같은 3층 — 계약(protocol), 서버 구현(core), 데스크톱 어댑터(AW). orchestration은 규모가 커서 core `application/orchestration/` 하위 모듈로 묶는다. AW에는 command 호환·창 전달·MCP transport만 남는다.

## 단계 (tasks 입력)

1. **Foundation**: protocol scope·DTO·operation id·스트림 구독 가능화, core로 orchestration 도메인·포트·저장소 이동(+ aggregate lock, `read/update`), 묶임·역할·작업 영역 lock, run 스트림 소유 기록, 과도기 접근자 정리 준비. 저장소 동시성 테스트(R1)를 먼저 작성(red → green).
2. **US1**(데스크톱 18): 서비스 이동·창 label → 작업대 묶임, handler 18 + `run.replay`, 작업대 닫기 hook, AW compat 18(`boundWindowLabel` 재구성), fixture.
3. **US2**(agent 16): 역할 판정, handler 16, `EngineAgentWorker`(자식 기동), MCP 도구 → `Workbench.call`, capability registry 축소, fixture.
4. **US3**(스트림): 묶임 스트림 발행·구독 권한·데스크톱 전달(삽입 경로 하나), run 구독 권한 보강, 스트림 테스트.
5. **US4**(창 없는 후처리): worktree 감시·과제 실패, scheduler, 알림 전달·복구 흐름 core, AW `RunTerminalHook`·`release_window`·scheduler 제거, 과도기 통로 제거 확인.
6. **US5**(계약): OpenAPI·describe·TS alias·drift.
7. **Polish**: 문서·ADR·전체 게이트·SC 증거.

## Complexity Tracking

위반 없음.

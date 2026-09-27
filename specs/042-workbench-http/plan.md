# Implementation Plan: Workbench HTTP/WebSocket 어댑터 (042, 3단계)

**Branch**: `042-workbench-http` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/042-workbench-http/spec.md`

## Summary

운영용 HTTP/WebSocket 어댑터를 새 크레이트 `crates/workbench-server`에 만든다. 이 크레이트는 `workbench-protocol`의 `Workbench` trait만 알고, 인증·서버 정보는 포트로 주입받는다. AW는 기동할 때 이 router를 **자기 `WorkbenchRuntime`과 같은 인스턴스**에 붙여 `127.0.0.1:<임의 포트>`로 연다. 인증 규칙은 다음과 같다: 모든 요청 bearer(짧은 데스크톱 토큰 / MCP 실행 토큰 → agent), 정확 Host·Origin, CORS allowlist, 30초 1회용 WS 표, 1 MiB 본문 상한, 토큰 비기록. 계약·이벤트 suite는 테스트 전용 harness 대신 이 운영 router로 두 경로 parity를 증명한다. 변경 operation은 중단·재시작 판정 증거가 있을 때만 연다. 증거가 없는 5개 영속 operation과 세대 범위·orchestration 변경의 "재시작 뒤 같은 키 재시도" 증거를 이번에 추가한다(research R13). MCP `origin_allowed` 접두사 결함을 같은 정확 일치 정책으로 고친다. 화면은 바꾸지 않는다(4단계).

## Technical Context

**Language/Version**: Rust 2021(core·protocol·새 server 크레이트), Rust 2024(AW src-tauri), TypeScript 5(생성 타입만)

**Primary Dependencies**: axum 0.7(`ws`), tower-http 0.5(`cors`, `limit`) — 이미 lock에 있음, tokio 1, sha2 0.10, uuid 1, serde_json. 새 major 없음.

**Storage**: 없음(토큰·표는 메모리). 기존 ledger·JSON 저장소를 그대로 쓴다.

**Testing**: `cargo test`(계약 suite·이벤트 suite를 운영 router로), 새 `workbench-server` 단위·통합 테스트(인증·출처·표·본문 상한·기록), core 중단·재시작 증거 테스트, AW 단위(MCP 출처, 연결 command), `pnpm run test`·`check-types`, 앱 스모크(quickstart §3 두 증거)

**Target Platform**: macOS(개발·스모크), Windows·Linux는 출처 목록만 포함(실측은 4단계 E2E)

**Project Type**: desktop-app + 공유 Rust 크레이트

**Performance Goals**: 로컬 루프백 호출 지연은 in-process 대비 체감 차이 없음(계약 suite 시간이 크게 늘지 않을 것), WS 경계 경합 1,000회 시험이 수 초 안에 끝남

**Constraints**: 127.0.0.1만 bind, 화면 diff 0, `acp-agent-core`·`agent-client` diff 0, 토큰·표 비기록, `CARGO_INCREMENTAL=0`(디스크)

**Scale/Scope**: operation 85개 전부 네트워크 노출(증거 조건 충족 뒤), 데스크톱 토큰 상한 256, 표 상한 1,024

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Monorepo Boundary First**: 새 크레이트는 `crates/workbench-server`(공유 Rust). AW는 조립·자격 증명 연결만. 화면·`acp-agent-core`·`agent-client` 불변 — 통과.
- **II. Frontend layering(FSD)**: 화면 변경 없음. 생성 타입만 갱신 — 통과.
- **III. Hexagonal Tauri Backend**: router는 inbound 어댑터(`Workbench` trait만 호출). 인증 해석·서버 정보는 포트(`CredentialResolver`·`ServerInfo`), 구현은 조립(AW)·테스트가 주입. AW의 새 command(`get_workbench_connection`)는 inbound, 발급기 연결은 infrastructure — 통과.
- **IV. Shared core vs UI**: 순수 Rust만 공유 — 통과.
- **V. Persistence and safety**: 새 영속 없음, 저장 주체 하나. 보안 규칙은 contracts §1–§7. 변경 공개 조건 R13 — 통과.
- **VI. Documentation**: `docs/workbench-seam.md` 네트워크 절, 연구 문서 각주, server 크레이트 ADR 2건 — 통과.

설계 뒤 재검사: 위와 같음(위반 없음, Complexity Tracking 비움).

## Project Structure

### Documentation (this feature)

```text
specs/042-workbench-http/
├── plan.md
├── research.md          # R1–R16 (R13 변경 공개 조건·중단 증거 표)
├── data-model.md
├── quickstart.md
├── contracts/
│   └── workbench-http.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/workbench-server/                 # 신설
├── Cargo.toml                           # workbench-protocol, axum, tower-http, tokio, sha2, uuid, serde_json
├── docs/adr/0001-…, 0002-…
└── src/
    ├── lib.rs                           # ServerConfig, build_router, serve(listener, router, shutdown)
    ├── auth.rs                          # CredentialResolver 포트, DesktopTokenIssuer(메모리), 합성 resolver
    ├── origin.rs                        # OriginPolicy(정확 일치), Host 검사 — AW MCP도 사용
    ├── tickets.rs                       # EventTicketStore(30초 1회용, 원자적 take)
    ├── handshake.rs                     # ServerInfo 포트, 협상
    ├── routes/{calls,events,health,openapi}.rs
    └── access_log.rs                    # 기록 sink(비밀 없음)

crates/workbench-core/tests/
├── support/http_harness.rs              # 운영 router 래퍼로 교체(고정 토큰 resolver, 표 흐름)
├── contract_suite.rs / event_contract_suite.rs   # 그대로(경로만 운영 router)
├── us1_crash_points.rs (또는 새 파일)   # project.update/delete, savedPrompt.update, goal.update/clear 중단 판정
└── restart_retry.rs                     # 세대 범위·orchestration 재시작 뒤 같은 키 재시도(+HTTP 한 번)

crates/workbench-server/tests/
├── security.rs                          # Host·Origin·토큰·표·본문 상한·기록 비노출
├── ws_boundary_race.rs                  # 표 구독 경계 경합 1,000회
└── mixed_paths.rs                       # in-process + HTTP 동시 변경 손실 0

apps/agentic-workbench/src-tauri/src/
├── lib.rs                               # router 조립·기동·종료, WorkbenchHttpState
├── infrastructure/workbench_http.rs     # 합성 resolver(데스크톱 발급기 + MCP CapabilityRegistry), 허용 출처, 진단·probe(debug 전용)
├── inbound/tauri_commands.rs            # get_workbench_connection, report_http_probe(debug)
└── infrastructure/mcp/title_tool.rs     # origin_allowed → workbench_server::origin::OriginPolicy
```

**Structure Decision**: 네트워크 어댑터는 새 공유 크레이트 `crates/workbench-server`(R1). core는 HTTP 운영 의존을 갖지 않는다(dev-dependency만). AW는 조립만 한다.

## Phases (tasks 입력)

1. **Setup**: 크레이트 생성·workspace 등록, 기준선 게이트 기록.
2. **Foundational**: 포트(`CredentialResolver`·`ServerInfo`), `OriginPolicy`, `DesktopTokenIssuer`, `EventTicketStore`, 기록 sink, router 골격(`/health/live`), problem 응답.
3. **US1(P1)**: `/v1/calls`·handshake·protocol 헤더, core harness를 운영 router로 교체 → 계약 suite parity, 경로 혼합 동시 변경. **R13 증거 테스트를 공개 전에**: 영속 5개 중단 판정, 세대 범위·orchestration 재시작 뒤 재시도.
4. **US2(P2)**: 표 발급·WS(hello→구독→프레임), 이벤트 suite parity, 경계 경합 1,000회, 재연결.
5. **US3(P3)**: Host·Origin·CORS·본문 상한 보안 테스트, MCP `origin_allowed` 교체.
6. **US4(P4)**: AW 조립(합성 resolver, MCP 토큰 → agent, 폐기 즉시 반영), `get_workbench_connection`, 기동 실패 허용·종료 정리.
7. **US5(P5)**: `/health/ready`·`/openapi.json`(커밋 파일과 같음).
8. **Polish**: 진단·WebView probe(debug 전용), 앱 스모크 두 증거, docs·ADR, 게이트, SC 증거, PR 초안.

## Complexity Tracking

없음.

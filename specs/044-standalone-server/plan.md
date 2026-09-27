# Implementation Plan: Workbench 독립 서버 분리 (044, 5단계 첫 증분)

**Branch**: `044-standalone-server` | **Date**: 2026-09-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/044-standalone-server/spec.md`

## Summary

Workbench 런타임·HTTP/WS 어댑터·MCP 서버·launch decorator를 새 crate `workbench-host`에 모으고, 새 앱 `apps/agentic-workbench-server`가 독립 composition root(`main`)로 이를 띄운다. 데스크톱(AW)은 기본 모드에서 앱 안에 런타임을 두지 않는다. 안내 파일로 서버를 찾거나 띄우고(`ensure`), 소유자 자격 증명으로 임대와 창 토큰을 받아 043 화면을 그대로 연결한다. 서버는 다음을 갖는다:
- 단일 writer(소유 잠금·시작 잠금·원자적 안내 파일)
- 인증된 버전 확인·준비·시작 복구
- 서빙→비우기→정지 상태, 임대·유휴, 정지 세 방식
- 비우기 입구 분류(Q/C/K/N — 새 작업과 끝내는 제어를 구분하고, 교환 전달을 조건부로 이어 가기)

앱 종료는 run을 취소하지 않는다. 작업대 닫기는 사용자가 창을 닫으려 한 경우에만 한다(R8, 실제 종료 이벤트 spike로 확정). #207(닫힌 작업대의 멱등 기록 부활)을 tombstone으로 고친다.

## Technical Context

**Language/Version**: Rust 1.98(workspace), TypeScript 5(AW 화면)

**Primary Dependencies**:
- 기존: tokio, axum(042), `workbench-core`·`workbench-server`·`workbench-protocol`, Tauri 2.
- 새로 쓰는 것:
  - 표준 라이브러리 파일 잠금(`File::try_lock`)
  - `reqwest`(default-features 끔, `json`)를 `workbench-host`의 일반 의존으로 — `ensure`의 loopback 확인용
  - 유닉스 권한(`std::os::unix::fs::PermissionsExt`)
  - `CommandExt::process_group`

**Storage**: 오늘의 데이터 디렉터리·도메인 JSON·ledger SQLite를 형식 변경 없이 연다. 새 파일은 `workbench/server/{owner.lock,startup.lock,server.json,server.log}`

**Testing**:
- 단위: core `drain_class`, EpochIdempotency tombstone, 판정 함수 `window_close_intent`
- host 통합: 실제 조립 + HTTP, wait-stop 실제 경로
- 서버 바이너리 프로세스 시험: 동시 시작·강제 kill 복구·권한·유휴·정지
- AW 화면·통합 시험(043 재사용, `continuation`)
- 실제 앱 스모크: 043 probe + `quit` 시나리오, 개발·배포 출처

**Target Platform**: macOS Apple Silicon 우선. Linux는 서버 프로세스 시험만 가능한 범위로 확인하고 Windows는 미검증 목록에 둔다.

**Project Type**: 데스크톱 앱 + 로컬 서버 프로세스

**Performance Goals**: 서버 강제 종료 뒤 복구·준비 5초 이내(SC-003). `ensure`가 새 서버를 띄우는 데 20초 상한.

**Constraints**:
- 단일 writer(데이터 디렉터리당 서버 하나)
- 안내 파일·자격 증명 0600
- 앱 종료가 run을 취소하지 않음
- 043 화면 계약·오류 문구 유지

**Scale/Scope**:
- 새 operation 8개: `server.status`, `server.stop`, `lease.acquire`·`renew`·`release`, `desktop.issueWindowToken`, `desktop.retireWindow`, `bench.list`
- `run.sendPrompt` 입력 필드 1개
- 새 crate 1, 새 앱 1

## 5단계 완료 기준 추적 (spec 표 유지)

| 기준 | 044에서 하는 것 | 044 완료 조건(검증) | 다음 증분(미완료로 남김) |
|---|---|---|---|
| (a) 종료 뒤 지속 | 앱 종료 ≠ 작업대 닫기, 소유자 주체, `bench.list`, 서버 소유 실행(A-turn·엔진 대기열·T-start·알림 전달기, R14) | 관측한 종료 경로마다 실제 앱 종료 → PID 소멸 → 소유자 클라이언트로 조회·출력·취소(개발·배포 출처) | CLI(6단계), 다시 연 데스크톱의 재부착 화면, **교환 전달의 서버 소유**(오늘은 창 원장·패널 대기열이 라우팅·전송 — 데스크톱이 없으면 새 교환이 전달되지 않음, R14) |
| (b) 독립 composition root | `workbench-host` + `apps/agentic-workbench-server` | 데스크톱 없이 서버 단독 기동·run·MCP | — |
| (c) 단일 writer·생명주기 | 잠금·안내 파일·ensure·복구·상태 기계·임대·유휴·정지 | 서버 프로세스 시험 전부, 실제 앱 연결 | — |
| (d) 프로세스 트리 가두기 | 오늘 수준 유지 | — | 공통 supervisor, 강제 종료 뒤 잔여 자식 정리 |
| (e) 데이터 이전·복원·호환 | 같은 형식으로 열기, 저장 형식 버전 확인 | 모르는 형식 거절 시험 | 백업·복원, 단계적 이전 |
| (f) 배포·업데이트 | 개발·디버그 빌드의 실행 파일 탐색 규칙 | 배포 출처 디버그 번들 + 옆에 둔 서버로 스모크 | 설치본 포함·서명·공증, 버전별 캐시, 업데이트 preflight |
| #207 | tombstone | 결정적 재현(수정 전 실패) + 원 시험 반복 기록 | — |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS.
  - 조립은 두 소비자(서버 앱, AW embedded)가 쓰므로 `crates/workbench-host`에 둔다.
  - 서버 `main`은 `apps/agentic-workbench-server`에 둔다.
  - AW는 앱 간 import 없이 crate만 쓴다(research R1).
- **Feature-Sliced Frontend Architecture**: PASS.
  - 연결 실패 화면은 `app`(부팅) + `shared/ui`.
  - 교환 `continuation`은 `features/agent-run/model`과 `entities/agent-run/api`.
  - 제목 적용은 `shared/lib` 소비처(`app`).
- **Hexagonal Tauri Backend Architecture**: PASS.
  - Tauri command는 host/HTTP 클라이언트에 위임한다.
  - 데스크톱 수명 판정 `window_close_intent`는 순수 함수(application), Tauri 이벤트 연결은 infrastructure.
  - host crate도 core의 port(DesktopBridge·RunLaunchDecorator)를 구현하는 infrastructure다.
- **Shared Core Before Shared UI**: N/A(공유 UI 없음).
- **Atomic Cross-App Verification**: PASS.
  - `workbench-core`·`workbench-protocol`·`workbench-server`·새 `workbench-host`가 바뀐다.
  - AW Rust 시험·check-types·pnpm test·통합 suite를 돌리고, `workbench-client` 생성물(operation map·kinds)을 재생성·검사한다.
  - 다른 앱은 이 crate들을 쓰지 않는다(확인 대상: `Cargo.toml` 의존 검색).
- **Documentation and Storybook**:
  - `docs/workbench-seam.md`에 "독립 서버(044)" 절을 둔다.
  - ADR 두 건: 독립 서버·소유자 주체, 앱 종료 ≠ 작업대 닫기.
  - `CONTEXT.md` 용어: 서버 인스턴스·소유자 주체·임대·비우기 분류.
  - 연결 실패 화면 Storybook 이야기.
- **Testing and Safety**:
  - 안내 파일 권한, 소유자 전용 op 거절, 창 토큰 폐기, 작업대 소유 우회가 소유자에게만 있음.
  - 비우기 분류 대조, 실제 경로 wait-stop과 대조 변이.
  - #207 결정적 재현.

## Project Structure

### Documentation (this feature)

```text
specs/044-standalone-server/
├── plan.md
├── research.md            # R1–R13, R7-check·R8-spike 결과 기록 자리
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── drain-classification.md
│   ├── server-lifecycle.md
│   └── desktop-client.md
└── tasks.md               # /speckit-tasks
```

### Source Code (repository root)

```text
crates/workbench-protocol/src/
├── call.rs, operations/{server,lease,desktop}.rs   # 새 operation 8개, run.sendPrompt continuation
└── principal.rs                                    # PrincipalKind::Owner, Scope server:admin
crates/workbench-core/src/application/
├── drain.rs (신규)          # DrainClass, drain_class, ActiveWork, 입구 판정
├── epoch_idempotency.rs     # closed_benches tombstone (#207)
├── authorization.rs         # 소유자 우회
└── workbench_runtime.rs     # 입구 판정·상태 참조·active_work()
crates/workbench-server/src/  # server.status/stop 등은 core operation. 서버는 상태 게이트(stopping 503)만
crates/workbench-host/ (신규)
├── src/assembly.rs          # 런타임 + HTTP 상태(발급기·표·소유자 resolver) + MCP
├── src/mcp/                 # AW src-tauri/infrastructure/mcp에서 이동
├── src/launch.rs            # McpLaunchDecorator (창 무관)
├── src/lifecycle/{lock,descriptor,ensure,state,lease,idle}.rs
└── tests/                   # 실제 조립 HTTP·wait-stop·ensure
apps/agentic-workbench-server/ (신규)
├── Cargo.toml, src/main.rs  # serve/ensure/status/stop
└── tests/process.rs         # 바이너리 프로세스 시험
apps/agentic-workbench/src-tauri/src/
├── lib.rs                   # 모드 선택(external/embedded), 외부 모드 종료 경로
├── infrastructure/server_client.rs (신규)  # ensure·임대·창 토큰·retire (host 사용)
├── infrastructure/window_lifecycle.rs       # closeIntent·quitting, retireWindow 호출
├── application/window_close_intent.rs (신규) # 순수 판정
└── inbound/tauri_commands.rs                # get_workbench_connection·ensure_window_bench 외부 모드, apply_window_title
apps/agentic-workbench/src/
├── app/bootstrap-transport.ts  # 외부 모드 대체 없음 → 연결 실패 상태
├── shared/ui/connection-failure.tsx (신규)
├── features/agent-run/model/exchange-reconciler.ts + entities/agent-run/api  # continuation
└── app/App.tsx               # 제목 이벤트 → apply_window_title
```

**Structure Decision**: 조립을 공유 crate로, 실행 파일은 앱으로 둔다. AW는 thin client가 되고, embedded 모드는 8단계 제거 대상으로 남는다.

## 구현 순서(단계)

1. **선행 확인**: R8-spike(실제 종료 이벤트 순서), R7-check(대기 자식 명령 전달 경로). 결과를 research에 기록하고 설계를 확정한다.
2. **#207**: 결정적 재현 시험(red) → tombstone → green, 원 시험 반복.
3. **protocol·core**: 새 operation·주체·scope, `drain_class` + 입구 판정(분류 대조 시험 먼저), `ActiveWork`, 소유자 우회.
4. **host crate**: 조립 이동(MCP·decorator 포함), 시험 host를 host 조립으로.
5. **생명주기**: 잠금·안내 파일·ensure·상태 기계·임대·유휴·정지(프로세스 시험 먼저).
6. **서버 앱**: `main`.
7. **데스크톱**: 모드, server_client, 창 수명(`window_close_intent`), 종료 경로, command 외부 모드, 화면(연결 실패·제목·continuation).
8. **실제 경로 wait-stop 시험**.
9. **실제 앱 스모크**: 043 스모크 외부 모드 + `quit` 시나리오(관측 경로마다) + 창 닫기 대조, 개발·배포 출처.
10. **문서**: ADR 두 건, seam 문서, CONTEXT, Storybook.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| embedded 모드 유지(코드 경로 둘) | 개발·시험 편의와 8단계까지의 호환 경로 | 지금 지우면 compat 제거(8단계)가 이 증분에 섞여 범위가 두 배가 된다. embedded도 같은 소유 잠금을 잡아 단일 writer는 지킨다 |
| 조건부 분류 K | 교환 전달이 새 prompt 모양이지만 이미 약속된 작업이다 | K 없이는 wait-stop 중 교환이 영원히 남거나(N), 모든 prompt를 허용해 비우기가 끝나지 않는다(C) |
